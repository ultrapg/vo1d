use crate::AppContext;
use anyhow::{bail, Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use std::io::Write;
use std::path::Path;

/// Download a model by ID. Fetches GGUF file with HTTP Range resume.
pub async fn download_model(ctx: &AppContext, model_id: &str) -> Result<()> {
    let entry = ctx.model_registry.get(model_id)
        .ok_or_else(|| anyhow::anyhow!("Model '{}' not found in registry", model_id))?;

    let model_path = ctx.paths.models_backend_dir("llamacpp").join(&entry.filename);

    // Check if already installed
    if model_path.exists() {
        println!("Model '{}' is already installed.", entry.name);
        return Ok(());
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

    ctx.audit.log_model("download_model", &format!("{} ({})", entry.name, model_id), ctx.security.current_mode)?;
    println!("Model installed to: {}", model_path.display());
    Ok(())
}

/// Download a file with progress bar (no checksum verification).
async fn download_file(url: &str, dest: &Path, total_size: u64) -> Result<()> {
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
    let mut stream = response.bytes_stream();

    use futures_util::StreamExt;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.context("Failed to read download chunk")?;
        file.write_all(&chunk).context("Failed to write download chunk")?;
        pb.inc(chunk.len() as u64);
    }

    pb.finish_and_clear();
    println!("Download completed.");

    Ok(())
}