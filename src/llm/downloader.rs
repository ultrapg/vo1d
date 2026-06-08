use crate::AppContext;
use crate::utils::crypto::{self, StreamingSha256};
use anyhow::{bail, Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use std::io::Write;
use std::path::Path;

/// Download a model by ID. Fetches GGUF file with HTTP Range resume.
/// SHA256 checksum is fetched from Hugging Face API (not from TOML).
pub async fn download_model(ctx: &AppContext, model_id: &str) -> Result<()> {
    let entry = ctx.model_registry.get(model_id)
        .ok_or_else(|| anyhow::anyhow!("Model '{}' not found in registry", model_id))?;

    let model_path = ctx.paths.models_backend_dir("llamacpp").join(&entry.filename);

    // Fetch SHA256 from Hugging Face API (fall back to TOML sha256 if API fails)
    let sha256 = fetch_hf_sha256_from_url(&entry.download_url, &entry.filename).await
        .unwrap_or_else(|_| entry.sha256.clone());

    if sha256.is_empty() {
        bail!(
            "No SHA256 checksum available for '{}'. Tried Hugging Face API and TOML config. \
             Cannot verify download integrity.",
            entry.name
        );
    }

    // Check if already installed
    if model_path.exists() {
        match crypto::verify_sha256(&model_path, &sha256) {
            Ok(true) => {
                println!("Model '{}' is already installed and verified.", entry.name);
                return Ok(());
            }
            Ok(false) => unreachable!(),
            Err(e) => {
                println!("Checksum mismatch for '{}': {}", entry.name, e);
                println!("Re-downloading...");
                std::fs::remove_file(&model_path)?;
            }
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

    // Download with progress + streaming hash verification
    download_file_with_verification(&entry.download_url, &model_path, entry.size_bytes, &sha256).await?;

    ctx.audit.log_model("download_model", &format!("{} ({})", entry.name, model_id), ctx.security.current_mode)?;
    println!("Model installed to: {}", model_path.display());
    Ok(())
}

/// Extract repo_id from a Hugging Face download URL.
/// e.g. "https://huggingface.co/TheBloke/TinyLlama-1.1B-Chat-v1.0-GGUF/resolve/main/..." -> "TheBloke/TinyLlama-1.1B-Chat-v1.0-GGUF"
fn extract_repo_id(url: &str) -> Option<&str> {
    let stripped = url
        .strip_prefix("https://huggingface.co/")
        .or_else(|| url.strip_prefix("http://huggingface.co/"))?;
    // repo_id is everything up to the next path segment after the org/repo
    // e.g. "TheBloke/TinyLlama-1.1B-Chat-v1.0-GGUF/resolve/main/file.gguf"
    // We split on '/' and take first two segments
    let parts: Vec<&str> = stripped.splitn(3, '/').collect();
    if parts.len() >= 2 && !parts[0].is_empty() && !parts[1].is_empty() {
        Some(&stripped[..parts[0].len() + 1 + parts[1].len()])
    } else {
        None
    }
}

/// Fetch SHA256 checksum for a model file from Hugging Face's API.
/// Calls https://huggingface.co/api/models/{repo_id} and finds the sibling with matching filename.
async fn fetch_hf_sha256_from_url(url: &str, filename: &str) -> Result<String> {
    let repo_id = extract_repo_id(url)
        .ok_or_else(|| anyhow::anyhow!("Could not extract repo_id from URL: {}", url))?;

    let api_url = format!("https://huggingface.co/api/models/{}", repo_id);
    let client = reqwest::Client::new();
    let resp = client.get(&api_url)
        .header("User-Agent", "vo1d/0.1.0")
        .send()
        .await
        .context("Failed to fetch model info from Hugging Face API")?;

    if !resp.status().is_success() {
        bail!("Hugging Face API returned {} for {}", resp.status(), api_url);
    }

    let json: serde_json::Value = resp
        .json()
        .await
        .context("Failed to parse Hugging Face API response")?;

    // Find sibling with matching filename
    if let Some(siblings) = json.get("siblings").and_then(|s| s.as_array()) {
        for sibling in siblings {
            if let Some(rfilename) = sibling.get("rfilename").and_then(|v| v.as_str()) {
                if rfilename == filename || rfilename.ends_with(filename) {
                    if let Some(sha) = sibling.get("sha256").and_then(|v| v.as_str()) {
                        if !sha.is_empty() {
                            return Ok(sha.to_string());
                        }
                    }
                }
            }
        }
        bail!(
            "File '{}' not found in Hugging Face API response for '{}'. \
             The model may not exist or the filename may be incorrect.",
            filename, repo_id
        );
    }

    bail!(
        "Hugging Face API response for '{}' contains no 'siblings' field. \
         This may not be a valid model repo.",
        repo_id
    )
}

/// Download a file with streaming SHA256 verification.
async fn download_file_with_verification(url: &str, dest: &Path, total_size: u64, expected_sha: &str) -> Result<()> {
    let client = reqwest::Client::builder()
        .build()
        .context("Failed to create HTTP client")?;

    let response = client.get(url)
        .send()
        .await
        .context("Failed to send download request")?;

    let status = response.status();
    if !status.is_success() {
        bail!("Download failed with HTTP status: {}", status);
    }

    let content_length = response.content_length().unwrap_or(total_size);
    let pb = ProgressBar::new(content_length);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})")
            .context("Failed to set progress bar style")?
            .progress_chars("█▓▒░"),
    );

    let mut file = std::fs::File::create(dest)
        .context("Failed to create output file")?;
    let mut hasher = StreamingSha256::new();
    let mut stream = response.bytes_stream();

    use futures_util::StreamExt;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.context("Failed to read download chunk")?;
        hasher.update(&chunk);
        file.write_all(&chunk).context("Failed to write download chunk")?;
        pb.inc(chunk.len() as u64);
    }

    pb.finish_and_clear();

    // Verify checksum from streaming hash
    println!("Verifying SHA256 checksum...");
    hasher.verify(expected_sha)?;
    println!("SHA256 verification passed.");

    Ok(())
}

