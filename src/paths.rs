use std::path::{Path, PathBuf};

/// Project root: `INFSCALER_HOME`, otherwise walk up from the cwd and the
/// executable until `scripts/setup.sh` is found.
pub fn find_root() -> Option<PathBuf> {
    if let Ok(value) = std::env::var("INFSCALER_HOME") {
        let path = PathBuf::from(value);
        if path.join("scripts/setup.sh").is_file() {
            return Some(path);
        }
    }

    let mut starts = Vec::new();
    if let Ok(cwd) = std::env::current_dir() {
        starts.push(cwd);
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            starts.push(dir.to_path_buf());
        }
    }

    for start in starts {
        let mut cur = Some(start.as_path());
        for _ in 0..8 {
            let Some(dir) = cur else { break };
            if dir.join("scripts/setup.sh").is_file() {
                return Some(dir.to_path_buf());
            }
            cur = dir.parent();
        }
    }
    None
}

pub fn models_dir(root: &Path) -> PathBuf {
    root.join("models")
}

pub fn bin_dir(root: &Path) -> PathBuf {
    root.join("bin")
}
