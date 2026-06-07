use crate::core::{Config, Message};
use crate::utils::costs::Usage;
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub messages: Vec<Message>,
    pub usage: Usage,
    pub model: String,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SessionConfig {
    #[serde(default)]
    pub allow_all_commands: bool,
    #[serde(default)]
    pub allowed_commands: Vec<String>,
    #[serde(default)]
    pub allow_all_outside_access: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    #[serde(default)]
    pub allow_all_outside_access: bool,
    #[serde(default)]
    pub allowed_outside_paths: Vec<String>,
}

pub fn load_workspace_config() -> anyhow::Result<WorkspaceConfig> {
    let path = get_workspace_dir().join("workspace_config.json");
    if !path.exists() {
        return Ok(WorkspaceConfig::default());
    }
    let json = std::fs::read_to_string(path)?;
    let config = serde_json::from_str(&json).unwrap_or_default();
    Ok(config)
}

pub fn save_workspace_config(config: &WorkspaceConfig) -> anyhow::Result<()> {
    let dir = get_workspace_dir();
    if !dir.exists() {
        std::fs::create_dir_all(&dir)?;
    }
    let path = dir.join("workspace_config.json");
    let json = serde_json::to_string_pretty(config)?;
    std::fs::write(path, json)?;
    Ok(())
}

pub fn get_base_dir() -> PathBuf {
    dirs::home_dir()
        .map(|p| p.join(".routecode"))
        .unwrap_or_else(|| PathBuf::from(".routecode"))
}

pub fn find_project_root() -> PathBuf {
    let mut current = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    loop {
        if current.join(".git").exists() || current.join("ROUTECODE.md").exists() {
            return current;
        }

        if let Some(parent) = current.parent() {
            current = parent.to_path_buf();
        } else {
            // Fallback to CWD
            return std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        }
    }
}

pub fn get_workspace_dir() -> PathBuf {
    let root = find_project_root();
    let root_str = root.to_string_lossy().to_string();

    let mut hasher = DefaultHasher::new();
    root_str.hash(&mut hasher);
    let hash = format!("{:x}", hasher.finish());

    let folder_name = root
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "workspace".to_string());

    let safe_folder_name = folder_name.replace(|c: char| !c.is_alphanumeric(), "_");
    let workspace_id = format!("{}_{}", safe_folder_name, &hash[..8]);

    get_base_dir().join("workspaces").join(workspace_id)
}

pub fn is_path_outside_workspace(path_str: &str) -> bool {
    let root = find_project_root();
    let root_canon = root.canonicalize().unwrap_or(root.clone());

    let mut p_str = path_str;
    if p_str.starts_with("/workspace/") {
        p_str = &p_str[11..];
    } else if p_str.starts_with("/workspace") {
        p_str = &p_str[10..];
    }

    let path = PathBuf::from(p_str);
    let absolute_path = if path.is_absolute() {
        path
    } else {
        root.join(path)
    };

    let mut p = absolute_path;
    while !p.exists() {
        if let Some(parent) = p.parent() {
            p = parent.to_path_buf();
        } else {
            break;
        }
    }

    if p.exists() {
        if let Ok(canon) = p.canonicalize() {
            return !canon.starts_with(&root_canon);
        }
    }

    false
}

pub fn sanitize_session_name(name: &str) -> String {
    name.chars()
        .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
        .collect()
}

pub fn save_session(name: &str, session: &Session) -> anyhow::Result<()> {
    let safe_name = sanitize_session_name(name);
    if safe_name.is_empty() {
        return Err(anyhow::anyhow!("Invalid session name"));
    }

    let workspace_dir = get_workspace_dir();
    let session_dir = workspace_dir.join("sessions").join(&safe_name);

    if !session_dir.exists() {
        fs::create_dir_all(&session_dir)?;
    }

    let history_path = session_dir.join("history.json");
    let json = serde_json::to_string_pretty(session)?;
    fs::write(history_path, json)?;
    Ok(())
}

pub fn load_session(name: &str) -> anyhow::Result<Session> {
    let safe_name = sanitize_session_name(name);
    if safe_name.is_empty() {
        return Err(anyhow::anyhow!("Invalid session name"));
    }

    let workspace_dir = get_workspace_dir();

    let old_path = get_base_dir()
        .join("sessions")
        .join(format!("{}.json", safe_name));
    let new_path = workspace_dir
        .join("sessions")
        .join(&safe_name)
        .join("history.json");

    let path = if new_path.exists() {
        new_path
    } else {
        old_path
    };

    let json = fs::read_to_string(path)?;
    let session = serde_json::from_str(&json)?;
    Ok(session)
}

pub fn list_sessions() -> anyhow::Result<Vec<String>> {
    let workspace_dir = get_workspace_dir();
    let new_sessions_dir = workspace_dir.join("sessions");
    let old_sessions_dir = get_base_dir().join("sessions");

    let mut sessions = Vec::new();

    if new_sessions_dir.exists() {
        for entry in fs::read_dir(new_sessions_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() && path.join("history.json").exists() {
                if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
                    sessions.push(name.to_string());
                }
            }
        }
    }

    if old_sessions_dir.exists() {
        for entry in fs::read_dir(old_sessions_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "json") {
                if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                    if !sessions.contains(&name.to_string()) {
                        sessions.push(name.to_string());
                    }
                }
            }
        }
    }

    Ok(sessions)
}

pub fn load_session_config(name: &str) -> anyhow::Result<SessionConfig> {
    let safe_name = sanitize_session_name(name);
    if safe_name.is_empty() {
        return Err(anyhow::anyhow!("Invalid session name"));
    }

    let workspace_dir = get_workspace_dir();
    let config_path = workspace_dir
        .join("sessions")
        .join(&safe_name)
        .join("session_config.json");

    if !config_path.exists() {
        return Ok(SessionConfig::default());
    }

    let json = fs::read_to_string(config_path)?;
    let config = serde_json::from_str(&json).unwrap_or_default();
    Ok(config)
}

pub fn save_session_config(name: &str, config: &SessionConfig) -> anyhow::Result<()> {
    let safe_name = sanitize_session_name(name);
    if safe_name.is_empty() {
        return Err(anyhow::anyhow!("Invalid session name"));
    }

    let workspace_dir = get_workspace_dir();
    let session_dir = workspace_dir.join("sessions").join(&safe_name);

    if !session_dir.exists() {
        fs::create_dir_all(&session_dir)?;
    }

    let config_path = session_dir.join("session_config.json");
    let json = serde_json::to_string_pretty(config)?;
    fs::write(config_path, json)?;
    Ok(())
}

pub fn load_config() -> anyhow::Result<Config> {
    let path = get_base_dir().join("config.json");
    if !path.exists() {
        return Ok(Config::default());
    }
    let json = fs::read_to_string(path)?;
    let mut config: Config = serde_json::from_str(&json)?;
    config.normalize();
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
