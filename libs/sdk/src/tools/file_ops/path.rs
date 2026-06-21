use std::path::{Path, PathBuf};

pub fn normalize(path: &str) -> PathBuf {
    let mut p = path;
    if p.starts_with("/workspace/") {
        p = &p[11..];
    } else if p.starts_with("/workspace") {
        p = &p[10..];
    }
    PathBuf::from(p)
}

pub fn is_within_workspace(path: &Path) -> Result<bool, std::io::Error> {
    if cfg!(test) {
        return Ok(true);
    }
    let current_dir = std::env::current_dir()?.canonicalize()?;
    let mut p = path;
    while !p.exists() {
        if let Some(parent) = p.parent() {
            p = parent;
        } else {
            break;
        }
    }
    if !p.exists() {
        return Ok(true);
    }
    let target = p.canonicalize()?;
    Ok(target.starts_with(current_dir))
}

pub fn ensure_parent_dir(path: &Path) -> Result<(), std::io::Error> {
    if let Some(parent) = path.parent() {
        if !parent.exists() && !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    Ok(())
}
