use crate::core::{Config, Message};
use crate::utils::costs::Usage;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub messages: Vec<Message>,
    pub usage: Usage,
    pub model: String,
    pub timestamp: i64,
}

pub fn get_base_dir() -> PathBuf {
    dirs::home_dir()
        .map(|p| p.join(".routecode"))
        .unwrap_or_else(|| PathBuf::from(".routecode"))
}

pub fn save_session(name: &str, session: &Session) -> anyhow::Result<()> {
    let dir = get_base_dir().join("sessions");
    if !dir.exists() {
        fs::create_dir_all(&dir)?;
    }

    let path = dir.join(format!("{}.json", name));
    let json = serde_json::to_string_pretty(session)?;
    fs::write(path, json)?;
    Ok(())
}

pub fn load_session(name: &str) -> anyhow::Result<Session> {
    let path = get_base_dir()
        .join("sessions")
        .join(format!("{}.json", name));
    let json = fs::read_to_string(path)?;
    let session = serde_json::from_str(&json)?;
    Ok(session)
}

pub fn list_sessions() -> anyhow::Result<Vec<String>> {
    let dir = get_base_dir().join("sessions");
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut sessions = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "json") {
            if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                sessions.push(name.to_string());
            }
        }
    }
    Ok(sessions)
}

pub fn load_config() -> anyhow::Result<Config> {
    let path = get_base_dir().join("config.json");
    if !path.exists() {
        return Ok(Config::default());
    }
    let json = fs::read_to_string(path)?;
    let config = serde_json::from_str(&json)?;
    Ok(config)
}

pub fn save_config(config: &Config) -> anyhow::Result<()> {
    let dir = get_base_dir();
    if !dir.exists() {
        fs::create_dir_all(&dir)?;
    }
    let path = dir.join("config.json");
    let json = serde_json::to_string_pretty(config)?;
    fs::write(path, json)?;
    Ok(())
}
