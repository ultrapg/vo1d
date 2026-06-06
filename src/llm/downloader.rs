use crate::AppContext;
use crate::utils::crypto;
use anyhow::{Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use std::io::Write;
use std::path::Path;

/// Download a model by ID. Fetches GGUF file with HTTP Range resume.
pub async fn download_model(ctx: &AppContext, model_id: &str) -> Result<()> {
    let entry = ctx.model_registry.get(model_id)
        .ok_or_else(|| anyhow::anyhow!("Model '{}' not found in registry", model_id))?;

    // Check if already installed
    let model_path = ctx.paths.models_backend_dir("llamacpp").join(&entry.filename);
    if model_path.exists() {
        // Verify checksum if we have it
        if !entry.sha256.is_empty() {
            let valid = crypto::verify_sha256(&model_path, &entry.sha256)
                .context("Failed to verify model checksum")?;
            if valid {
                println!("Model '{}' is already installed and verified.", entry.name);
                return Ok(());
            } else {
                println!("Checksum mismatch for '{}'. Re-downloading...", entry.name);
                std::fs::remove_file(&model_path)?;
            }
        } else {
            println!("Model '{}' is already installed.", entry.name);
            return Ok(());
        }
    }

    // Confirm with user
    let size_mb = entry.size_bytes as f64 / 1_048_576.0;
    println!("Downloading {} ({}, {:.0} MB)", entry.name, entry.quantization, size_mb);
    println!("Size: {:.1} MB | URL: {}", size_mb, entry.download_url);
    println!("RAM required: {:.0} GB", entry.min_ram_gb);
    print!("Proceed with download? (Y/n): ");
    std::io::Write::flush(&mut std::io::stdout())?;

    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    if !input.trim().is_empty() && !input.trim().eq_ignore_ascii_case("y") {
        println!("Download cancelled.");
        return Ok(());
    }

    // Ensure backend directory exists
    let backend_dir = ctx.paths.models_backend_dir("llamacpp");
    std::fs::create_dir_all(&backend_dir)?;

    // Download with progress
    download_file(&entry.download_url, &model_path, entry.size_bytes).await?;

    // Verify checksum
    if !entry.sha256.is_empty() {
        println!("Verifying SHA256 checksum...");
        let valid = crypto::verify_sha256(&model_path, &entry.sha256)
            .context("Failed to verify checksum")?;
        if !valid {
            anyhow::bail!("SHA256 checksum mismatch! File may be corrupted.");
        }
        println!("SHA256 verification passed.");
    }

    ctx.audit.log_model("download_model", &format!("{} ({})", entry.name, model_id), ctx.security.current_mode)?;
    println!("Model installed to: {}", model_path.display());
    Ok(())
}

/// Download a file with HTTP Range resume support and progress bar.
async fn download_file(url: &str, dest: &Path, total_size: u64) -> Result<()> {
    let client = reqwest::Client::builder()
        .build()
        .context("Failed to create HTTP client")?;

    let part_path = dest.with_extension("gguf.part");

    // Check for existing partial download
    let downloaded = if part_path.exists() {
        std::fs::metadata(&part_path)?.len()
    } else {
        0
    };

    // Request with Range header for resume
    let mut request = client.get(url);
    if downloaded > 0 {
        request = request.header("Range", format!("bytes={}-", downloaded));
    }

    let response = request.send().await.context("Failed to send download request")?;
    let status = response.status();

    if !status.is_success() && status != reqwest::StatusCode::PARTIAL_CONTENT {
        anyhow::bail!("Download failed with status: {}", status);
    }

    let content_length = response.content_length().unwrap_or(total_size);
    let total = if downloaded > 0 { content_length + downloaded } else { content_length };

    // Progress bar
    let pb = ProgressBar::new(total);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})")
            .context("Failed to set progress bar style")?
            .progress_chars("█▓▒░"),
    );
    pb.set_position(downloaded);

    // Write to .part file
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&part_path)
        .context("Failed to open partial download file")?;

    let mut stream = response.bytes_stream();
    use futures_util::StreamExt;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.context("Failed to read download chunk")?;
        file.write_all(&chunk).context("Failed to write download chunk")?;
        pb.inc(chunk.len() as u64);
    }

    pb.finish_and_clear();
    println!("Download complete.");

    // Rename .part to final name
    std::fs::rename(&part_path, dest)
        .with_context(|| format!("Failed to rename partial to final: {} -> {}", part_path.display(), dest.display()))?;

    Ok(())
}
