use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

include!(concat!(env!("OUT_DIR"), "/defaults.rs"));

/// Write the default roles and pods into `dir/.agent-roles/`, and return the
/// written paths. Return `None` and write nothing if `.agent-roles` exists.
pub(crate) fn init(dir: &Path) -> Result<Option<Vec<PathBuf>>> {
    let root = dir.join(".agent-roles");
    // `symlink_metadata` so that a dangling symlink also counts as existing.
    if root.symlink_metadata().is_ok() {
        return Ok(None);
    }
    let mut written = Vec::new();
    for (relative, content) in DEFAULTS {
        let path = root.join(relative);
        fs::create_dir_all(path.parent().unwrap_or(&root))
            .with_context(|| format!("cannot create {}", root.display()))?;
        fs::write(&path, content).with_context(|| format!("cannot write {}", path.display()))?;
        written.push(path);
    }
    Ok(Some(written))
}
