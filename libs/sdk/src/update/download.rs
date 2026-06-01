use anyhow::{Context, Result};
use std::path::Path;
use tokio::io::AsyncWriteExt;

pub async fn download_file(url: &str, dest: &Path) -> Result<()> {
    let client = reqwest::Client::new();
    let resp = client.get(url).send().await?;
    let bytes = resp.bytes().await?;

    if let Some(parent) = dest.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    let mut file = tokio::fs::File::create(dest).await?;
    file.write_all(&bytes).await?;

    Ok(())
}

pub async fn download_checksum(url: &str) -> Result<String> {
    let client = reqwest::Client::new();
    let resp = client.get(url).send().await?;
    resp.text().await.map_err(Into::into)
}

pub fn verify_checksum(file_path: &Path, expected_checksum: &str) -> Result<bool> {
    let data = std::fs::read(file_path)
        .context("Failed to read downloaded file for checksum verification")?;
    let hash = sha256::digest(&data);
    Ok(hash == expected_checksum)
}

pub fn extract_checksum_for_asset(checksum_text: &str, asset_name: &str) -> Option<String> {
    for line in checksum_text.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 {
            let hash = parts[0];
            let name = parts[parts.len() - 1];
            if name == asset_name || name == format!("*{}", asset_name) {
                return Some(hash.to_string());
            }
        }
    }
    None
}
