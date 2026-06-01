use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateInfo {
    pub version: String,
    pub current_version: String,
    pub changelog: String,
    pub download_url: String,
    pub checksum_url: String,
    pub published_at: String,
    pub is_update_available: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubRelease {
    pub tag_name: String,
    pub name: String,
    pub body: String,
    pub published_at: String,
    pub assets: Vec<GitHubAsset>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubAsset {
    pub name: String,
    pub browser_download_url: String,
    pub size: u64,
}

pub fn get_platform_asset_name() -> Option<&'static str> {
    if cfg!(target_os = "windows") {
        Some("RouteCode-cli-windows-x86_64.exe")
    } else if cfg!(target_os = "macos") {
        if cfg!(target_arch = "aarch64") {
            Some("RouteCode-cli-macos-arm64")
        } else {
            Some("RouteCode-cli-macos-x86_64")
        }
    } else if cfg!(target_os = "linux") {
        Some("RouteCode-cli-linux-x86_64")
    } else {
        None
    }
}

pub fn get_platform_checksum_asset_name() -> Option<&'static str> {
    if cfg!(target_os = "windows") {
        Some("RouteCode-cli-windows-x86_64.exe.sha256")
    } else if cfg!(target_os = "macos") {
        if cfg!(target_arch = "aarch64") {
            Some("RouteCode-cli-macos-arm64.sha256")
        } else {
            Some("RouteCode-cli-macos-x86_64.sha256")
        }
    } else if cfg!(target_os = "linux") {
        Some("RouteCode-cli-linux-x86_64.sha256")
    } else {
        None
    }
}
