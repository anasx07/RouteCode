use super::types::{get_platform_asset_name, get_platform_checksum_asset_name, GitHubRelease, UpdateInfo};
use anyhow::{Context, Result};
use semver::Version;

pub async fn check_for_update(
    current_version: &str,
    repo: &str,
) -> Result<UpdateInfo> {
    let client = reqwest::Client::builder()
        .user_agent(format!("routecode-updater/{}", current_version))
        .build()?;

    let url = format!("https://api.github.com/repos/{}/releases/latest", repo);
    let resp = client.get(&url).send().await?;
    let release: GitHubRelease = resp.json().await
        .context("Failed to parse GitHub release JSON")?;

    let tag = release.tag_name.trim_start_matches('v');
    let latest_version = Version::parse(tag)
        .map_err(|e| anyhow::anyhow!("Failed to parse latest version '{}': {}", tag, e))?;
    let current = Version::parse(current_version)
        .map_err(|e| anyhow::anyhow!("Failed to parse current version '{}': {}", current_version, e))?;

    let is_update_available = latest_version > current;

    let platform_asset = get_platform_asset_name();
    let checksum_asset = get_platform_checksum_asset_name();

    let download_url = platform_asset
        .and_then(|name| {
            release.assets.iter().find(|a| a.name == name)
                .map(|a| a.browser_download_url.clone())
        })
        .unwrap_or_else(|| release.assets.first()
            .map(|a| a.browser_download_url.clone())
            .unwrap_or_default());

    let checksum_url = checksum_asset
        .and_then(|name| {
            release.assets.iter().find(|a| a.name == name)
                .map(|a| a.browser_download_url.clone())
        })
        .unwrap_or_else(|| {
            release.assets.iter()
                .find(|a| a.name == "checksums.txt")
                .map(|a| a.browser_download_url.clone())
                .unwrap_or_default()
        });

    Ok(UpdateInfo {
        version: release.tag_name,
        current_version: current_version.to_string(),
        changelog: release.body,
        download_url,
        checksum_url,
        published_at: release.published_at,
        is_update_available,
    })
}

pub fn should_check(last_check: f64, interval_hours: u64) -> bool {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64();
    let interval_secs = (interval_hours as f64) * 3600.0;
    (now - last_check) >= interval_secs
}

pub fn now_timestamp() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64()
}
