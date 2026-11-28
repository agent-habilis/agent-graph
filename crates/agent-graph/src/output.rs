//! Write an output file that the user names.

use std::io::Write as _;
use std::path::Path;
use std::{fs, io};

use anyhow::{Context, Result, bail};

/// Write `bytes` to a new file at `path`. An old file there is replaced. A
/// symbolic link is not followed: it is an error, so a link cannot send the
/// write to another file.
pub(crate) fn write_new(path: &Path, bytes: &[u8]) -> Result<()> {
    match path.symlink_metadata() {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            bail!(
                "{} is a symbolic link, and a write would go through it",
                path.display()
            );
        }
        Ok(metadata) if metadata.is_dir() => bail!("{} is a folder", path.display()),
        Ok(_) => {
            fs::remove_file(path).with_context(|| format!("cannot replace {}", path.display()))?;
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(error).with_context(|| format!("cannot read {}", path.display())),
    }
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .and_then(|mut file| file.write_all(bytes))
        .with_context(|| format!("cannot write {}", path.display()))
}
