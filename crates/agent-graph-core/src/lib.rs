mod frontmatter;
pub mod graph;
pub mod include;
mod markdown;
pub mod name;
pub mod node;
pub mod traits;

use std::path::PathBuf;

/// The home folder of the user. On Unix it is `$HOME`. On Windows it is
/// `USERPROFILE`, or the profile folder of the user when that is unset:
/// an agent started from PowerShell has no `HOME`.
#[must_use]
pub fn home() -> Option<PathBuf> {
    #[cfg(not(windows))]
    {
        std::env::var_os("HOME").map(PathBuf::from)
    }
    #[cfg(windows)]
    {
        std::env::home_dir()
    }
}
