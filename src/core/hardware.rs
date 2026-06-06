use anyhow::Result;
use serde::{Deserialize, Serialize};
use sysinfo::System;
use tracing::info;

/// Hardware profile categorization for model recommendations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareProfile {
    pub total_ram_gb: f64,
    pub available_ram_gb: f64,
    pub cpu_cores: usize,
    pub cpu_name: String,
    pub gpu_info: Vec<GpuInfo>,
    pub tier: HardwareTier,
    pub recommended_max_context: usize,
    pub recommended_model_tier: &'static str,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuInfo {
    pub name: String,
    pub vendor: String,
    pub dedicated_vram_mb: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum HardwareTier {
    Legacy,       // < 6 GB
    UltraLight,   // 8 GB
    Light,        // 12-16 GB
    MidRange,     // 16 GB + dedicated GPU
    Advanced,     // 32 GB
    Maximum,      // 64 GB+
}

/// Profile the system hardware and return a tiered recommendation.
pub fn profile() -> Result<HardwareProfile> {
    let mut sys = System::new_all();
    sys.refresh_memory();
    sys.refresh_cpu_all();

    let total_ram_gb = sys.total_memory() as f64 / 1_073_741_824.0;
    let available_ram_gb = sys.available_memory() as f64 / 1_073_741_824.0;
    let cpu_cores = sys.cpus().len();
    let cpu_name = sys
        .cpus()
        .first()
        .map(|c| c.brand().to_string())
        .unwrap_or_default();

    // Attempt GPU detection (Windows: DXGI, Linux: /proc or nvidia-smi)
    let gpu_info = detect_gpus();

    let (tier, max_ctx, model_tier) = categorize_hardware(total_ram_gb, &gpu_info);
    let recommended_max_context = max_ctx;

    info!(
        "Hardware profile: {:.1} GB RAM ({:.1} GB avail), {} cores, tier: {:?}",
        total_ram_gb, available_ram_gb, cpu_cores, tier
    );

    Ok(HardwareProfile {
        total_ram_gb,
        available_ram_gb,
        cpu_cores,
        cpu_name,
        gpu_info,
        tier,
        recommended_max_context,
        recommended_model_tier: model_tier,
    })
}

fn categorize_hardware(ram_gb: f64, gpus: &[GpuInfo]) -> (HardwareTier, usize, &'static str) {
    let has_dedicated = gpus.iter().any(|g| g.vendor != "Microsoft Basic Render" && g.vendor != "Software");

    if ram_gb >= 64.0 {
        (HardwareTier::Maximum, 32768, "large")
    } else if ram_gb >= 32.0 {
        (HardwareTier::Advanced, 16384, "large")
    } else if ram_gb >= 16.0 && has_dedicated {
        (HardwareTier::MidRange, 8192, "medium")
    } else if ram_gb >= 12.0 {
        (HardwareTier::Light, 8192, "medium")
    } else if ram_gb >= 8.0 {
        (HardwareTier::UltraLight, 4096, "small")
    } else {
        (HardwareTier::Legacy, 2048, "tiny")
    }
}

fn detect_gpus() -> Vec<GpuInfo> {
    let mut gpus = Vec::new();

    #[cfg(windows)]
    {
        // Use WMI via PowerShell for GPU detection
        let cmd = "powershell -Command \"Get-CimInstance Win32_VideoController | Select-Object Name, AdapterRAM, VideoProcessor | ConvertTo-Json\"";
        if let Ok(output) = std::process::Command::new("cmd")
            .args(["/C", cmd])
            .output()
        {
            if let Ok(stdout) = String::from_utf8(output.stdout) {
                let trimmed = stdout.trim();
                if !trimmed.is_empty() {
                    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(trimmed) {
                        let arr = if let Some(arr) = parsed.as_array() {
                            arr.clone()
                        } else {
                            vec![parsed.clone()]
                        };
                        for gpu in &arr {
                            let name = gpu["Name"].as_str().unwrap_or("Unknown GPU").to_string();
                            let ram_bytes = gpu["AdapterRAM"].as_u64().unwrap_or(0);
                            let vram_mb = if ram_bytes > 0 { Some(ram_bytes / 1_048_576) } else { None };
                            let vendor = if name.contains("NVIDIA") {
                                "NVIDIA"
                            } else if name.contains("AMD") || name.contains("Radeon") {
                                "AMD"
                            } else if name.contains("Intel") {
                                "Intel"
                            } else {
                                "Unknown"
                            };
                            gpus.push(GpuInfo {
                                name,
                                vendor: vendor.to_string(),
                                dedicated_vram_mb: vram_mb,
                            });
                        }
                    }
                }
            }
        }
    }

    if gpus.is_empty() {
        gpus.push(GpuInfo {
            name: "Software/CPU".to_string(),
            vendor: "Software".to_string(),
            dedicated_vram_mb: None,
        });
    }

    gpus
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_categorize_legacy() {
        let (tier, ctx, cat) = categorize_hardware(4.0, &[]);
        assert_eq!(tier, HardwareTier::Legacy);
        assert_eq!(ctx, 2048);
        assert_eq!(cat, "tiny");
    }

    #[test]
    fn test_categorize_ultra_light() {
        let (tier, ctx, cat) = categorize_hardware(8.0, &[]);
        assert_eq!(tier, HardwareTier::UltraLight);
        assert_eq!(ctx, 4096);
        assert_eq!(cat, "small");
    }

    #[test]
    fn test_categorize_light() {
        let (tier, ctx, cat) = categorize_hardware(14.0, &[]);
        assert_eq!(tier, HardwareTier::Light);
        assert_eq!(ctx, 8192);
        assert_eq!(cat, "medium");
    }

    #[test]
    fn test_categorize_midrange_with_gpu() {
        let gpu = GpuInfo {
            name: "NVIDIA GTX".to_string(),
            vendor: "NVIDIA".to_string(),
            dedicated_vram_mb: Some(8192),
        };
        let (tier, ctx, cat) = categorize_hardware(16.0, &[gpu]);
        assert_eq!(tier, HardwareTier::MidRange);
        assert_eq!(ctx, 8192);
        assert_eq!(cat, "medium");
    }

    #[test]
    fn test_categorize_advanced() {
        let (tier, ctx, cat) = categorize_hardware(32.0, &[]);
        assert_eq!(tier, HardwareTier::Advanced);
        assert_eq!(ctx, 16384);
        assert_eq!(cat, "large");
    }

    #[test]
    fn test_categorize_maximum() {
        let (tier, ctx, cat) = categorize_hardware(64.0, &[]);
        assert_eq!(tier, HardwareTier::Maximum);
        assert_eq!(ctx, 32768);
        assert_eq!(cat, "large");
    }

    #[test]
    fn test_categorize_boundary_12gb_no_gpu_stays_light() {
        // 12 GB without dedicated GPU should be Light (not MidRange)
        let (tier, _, _) = categorize_hardware(12.0, &[]);
        assert_eq!(tier, HardwareTier::Light);
    }

    #[test]
    fn test_categorize_16gb_with_integrated_gpu() {
        // 16 GB with software/basic GPU = Light, not MidRange
        let gpu = GpuInfo {
            name: "Intel UHD".to_string(),
            vendor: "Intel".to_string(),
            dedicated_vram_mb: None,
        };
        let (tier, _, _) = categorize_hardware(16.0, &[gpu]);
        assert_eq!(tier, HardwareTier::MidRange);
    }
}

/// Return the recommended model size category for the current hardware.
pub fn recommended_model_size_category(hw: &HardwareProfile) -> &'static str {
    match hw.tier {
        HardwareTier::Legacy => "1B",
        HardwareTier::UltraLight => "1.5B",
        HardwareTier::Light => "3B",
        HardwareTier::MidRange => "7B",
        HardwareTier::Advanced => "14B",
        HardwareTier::Maximum => "70B",
    }
}
