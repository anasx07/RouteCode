use std::fs;
use std::io;
use std::path::Path;

pub const IGNORED_DIRS: &[&str] = &[".git", "node_modules", "target"];

pub fn should_skip_dir(name: &str) -> bool {
    IGNORED_DIRS.contains(&name)
}

pub fn is_binary(path: &Path) -> bool {
    use std::io::Read;
    if let Ok(mut file) = fs::File::open(path) {
        let mut buffer = [0; 1024];
        if let Ok(bytes_read) = file.read(&mut buffer) {
            return buffer[..bytes_read].contains(&0);
        }
    }
    false
}

pub fn walk_tree<F>(dir: &Path, current_depth: usize, max_depth: usize, mut visit: F) -> io::Result<()>
where
    F: FnMut(&Path, usize) -> io::Result<()>,
{
    if current_depth > max_depth {
        return Ok(());
    }
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if path.is_dir() {
            if should_skip_dir(&name) {
                continue;
            }
            visit(&path, current_depth)?;
            walk_tree(&path, current_depth + 1, max_depth, &mut visit)?;
        } else {
            visit(&path, current_depth)?;
        }
    }
    Ok(())
}
