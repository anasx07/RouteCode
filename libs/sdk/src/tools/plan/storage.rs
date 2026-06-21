use std::fs;
use std::path::PathBuf;

/// Returns the root directory where all plans are stored:
/// `~/.routecode/plans/`. Created on first write.
pub fn plans_root() -> std::io::Result<PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Home directory not found",
        )
    })?;
    let root = home.join(".routecode").join("plans");
    fs::create_dir_all(&root)?;
    Ok(root)
}

/// Returns the per-session plan directory:
/// `~/.routecode/plans/{session_id}/`. Created on first write.
pub fn session_dir(session_id: &str) -> std::io::Result<PathBuf> {
    let dir = plans_root()?.join(session_id);
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// The next free plan slug for a session, scanning existing
/// `plan-1.md`, `plan-2.md`, … Returns the absolute path of a NOT-YET-
/// created file.
pub fn next_plan_path(session_id: &str) -> std::io::Result<PathBuf> {
    let dir = session_dir(session_id)?;
    let mut n = 1u32;
    loop {
        let candidate = dir.join(format!("plan-{}.md", n));
        if !candidate.exists() {
            return Ok(candidate);
        }
        n += 1;
        if n > u32::MAX - 1 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Too many plans in this session",
            ));
        }
    }
}

/// Write a plan markdown to disk. Returns the absolute path.
pub fn write_plan(
    session_id: &str,
    content: &str,
) -> std::io::Result<PathBuf> {
    let path = next_plan_path(session_id)?;
    fs::write(&path, content)?;
    Ok(path)
}

/// Read the most recent plan for a session. Returns `None` if no
/// plans exist.
pub fn read_latest_plan(
    session_id: &str,
) -> std::io::Result<Option<(PathBuf, String)>> {
    let dir = session_dir(session_id)?;
    let mut plans: Vec<PathBuf> = fs::read_dir(&dir)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.extension()
                .and_then(|s| s.to_str())
                == Some("md")
        })
        .collect();
    plans.sort();
    match plans.pop() {
        Some(p) => {
            let content = fs::read_to_string(&p)?;
            Ok(Some((p, content)))
        }
        None => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fake_session() -> String {
        format!(
            "test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        )
    }

    #[test]
    fn writes_and_reads_latest() {
        let sid = fake_session();
        let p1 = write_plan(&sid, "# Plan 1\nhello").unwrap();
        let p2 = write_plan(&sid, "# Plan 2\nworld").unwrap();
        assert_ne!(p1, p2);
        let (latest_path, latest_content) = read_latest_plan(&sid).unwrap().unwrap();
        assert_eq!(latest_path, p2);
        assert!(latest_content.contains("Plan 2"));
        // Cleanup
        let _ = std::fs::remove_dir_all(
            plans_root().unwrap().join(&sid),
        );
    }

    #[test]
    fn empty_session_has_no_plan() {
        let sid = fake_session();
        let result = read_latest_plan(&sid).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn slugs_are_session_isolated() {
        let s1 = fake_session();
        let s2 = format!("{}-other", s1);
        let _ = write_plan(&s1, "s1 plan").unwrap();
        let p2 = write_plan(&s2, "s2 plan").unwrap();
        let (_, content) = read_latest_plan(&s2).unwrap().unwrap();
        assert!(content.contains("s2 plan"));
        // Cleanup
        let _ = std::fs::remove_dir_all(
            plans_root().unwrap().join(&s1),
        );
        let _ = std::fs::remove_dir_all(
            plans_root().unwrap().join(&s2),
        );
        // p2 went out of scope, but the directory was already cleaned
        let _ = p2;
    }
}
