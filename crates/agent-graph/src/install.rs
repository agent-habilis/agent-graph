//! `graph install`: put a graph folder at `~/.agent-graph/graphs/<hash>`.
//!
//! The graph is staged in `~/.agent-graph/.staging/`, on the same file system,
//! and moved into place only when its hash is right, so a failed install
//! leaves nothing in `graphs/`, where `graph list` looks.

use std::path::{Component, Path, PathBuf};
use std::time::{Duration, SystemTime};
use std::{fs, io};

use agent_graph_core::graph;
use anyhow::{Context, Result, bail};
use tar::{Archive, EntryType};

/// A graph is a few text files. A tar over these limits is not a graph.
pub(crate) const MAX_BYTES: u64 = 16 * 1024 * 1024;
pub(crate) const MAX_ENTRIES: usize = 10_000;

/// A stage folder that no install touched for this long is left over from a
/// process that died.
const STALE_AFTER: Duration = Duration::from_hours(1);

/// A folder that is removed when it goes out of scope, unless it was moved.
struct Staging(PathBuf);

impl Drop for Staging {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn agent_graph_dir() -> Result<PathBuf> {
    match std::env::var_os("HOME") {
        None => bail!("`$HOME` is not set"),
        Some(home) if home.is_empty() => bail!("`$HOME` is empty"),
        Some(home) => Ok(PathBuf::from(home).join(".agent-graph")),
    }
}

fn graphs_dir() -> Result<PathBuf> {
    Ok(agent_graph_dir()?.join("graphs"))
}

fn staging_dir() -> Result<PathBuf> {
    Ok(agent_graph_dir()?.join(".staging"))
}

/// A new empty folder in the staging folder. It also removes the stage
/// folders that a dead process left.
fn stage(hash: &str) -> Result<Staging> {
    let base = staging_dir()?;
    fs::create_dir_all(&base).with_context(|| format!("cannot create {}", base.display()))?;
    remove_stale(&base);
    let path = base.join(format!("{hash}.tmp-{}", std::process::id()));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).with_context(|| format!("cannot create {}", path.display()))?;
    Ok(Staging(path))
}

fn remove_stale(base: &Path) {
    let Ok(entries) = fs::read_dir(base) else {
        return;
    };
    for entry in entries.flatten() {
        let age = entry
            .metadata()
            .and_then(|metadata| metadata.modified())
            .ok()
            .and_then(|time| SystemTime::now().duration_since(time).ok());
        if age.is_some_and(|age| age > STALE_AFTER) {
            let _ = remove_path(&entry.path());
        }
    }
}

fn remove_path(path: &Path) -> io::Result<()> {
    if path.symlink_metadata()?.is_dir() {
        fs::remove_dir_all(path)
    } else {
        fs::remove_file(path)
    }
}

/// Whether `<graphs>/<hash>` is there and holds what its name says.
fn is_installed(hash: &str) -> Result<bool> {
    let dest = graphs_dir()?.join(hash);
    Ok(dest.is_dir() && graph::hash(&dest).is_ok_and(|found| found == hash))
}

/// Fail unless the folder at `path` has the hash `expect`.
fn verify(path: &Path, expect: &str) -> Result<()> {
    let found = graph::hash(path).with_context(|| format!("cannot read {}", path.display()))?;
    if found != expect {
        bail!("hash mismatch: expected {expect}, got {found}");
    }
    Ok(())
}

/// Move the staged graph to `<graphs>/<hash>`. A folder that is already there
/// does not match its name, or the install would have stopped earlier, so it
/// is replaced.
fn publish(staging: Staging, hash: &str) -> Result<()> {
    let graphs = graphs_dir()?;
    fs::create_dir_all(&graphs).with_context(|| format!("cannot create {}", graphs.display()))?;
    let dest = graphs.join(hash);
    let aside = staging
        .0
        .with_file_name(format!("{hash}.old-{}", std::process::id()));
    let moved_aside = dest.symlink_metadata().is_ok() && fs::rename(&dest, &aside).is_ok();
    match fs::rename(&staging.0, &dest) {
        Ok(()) => {
            std::mem::forget(staging);
            if moved_aside {
                let _ = remove_path(&aside);
            }
            Ok(())
        }
        Err(error) => {
            if moved_aside {
                let _ = fs::rename(&aside, &dest);
            }
            Err(error).with_context(|| format!("cannot create {}", dest.display()))
        }
    }
}

/// Copy a graph folder to `~/.agent-graph/graphs/<hash>`. Returns the hash.
pub(crate) fn from_folder(folder: &Path) -> Result<String> {
    agent_graph_dir()?;
    let hash = graph::hash(folder).with_context(|| format!("cannot read {}", folder.display()))?;
    if is_installed(&hash)? {
        return Ok(hash);
    }
    let staging = stage(&hash)?;
    copy_dir(folder, &staging.0)?;
    set_modes(&staging.0)?;
    verify(&staging.0, &hash).context("the folder changed while it was copied")?;
    publish(staging, &hash)?;
    Ok(hash)
}

fn copy_dir(from: &Path, to: &Path) -> Result<()> {
    for entry in fs::read_dir(from).with_context(|| format!("cannot read {}", from.display()))? {
        let path = entry?.path();
        let name = path.file_name().expect("a folder entry has a name");
        if graph::is_apple_double(Path::new(name)) {
            continue;
        }
        let target = to.join(name);
        if path.is_dir() {
            fs::create_dir_all(&target)?;
            copy_dir(&path, &target)?;
        } else {
            fs::copy(&path, &target).with_context(|| format!("cannot copy {}", path.display()))?;
        }
    }
    Ok(())
}

/// Files get 0644 and folders 0755, whatever the source had: the hash does not
/// cover modes, so a mode must not decide who can read the graph.
#[cfg(unix)]
fn set_modes(dir: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    fs::set_permissions(dir, fs::Permissions::from_mode(0o755))?;
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            set_modes(&path)?;
        } else {
            fs::set_permissions(&path, fs::Permissions::from_mode(0o644))?;
        }
    }
    Ok(())
}

#[cfg(not(unix))]
fn set_modes(_dir: &Path) -> Result<()> {
    Ok(())
}

/// Extract a tar file of a graph folder to `~/.agent-graph/graphs/<expect>`.
/// The folder stays only if its hash is `expect`. Returns the hash.
pub(crate) fn from_tar(file: &Path, expect: &str) -> Result<String> {
    agent_graph_dir()?;
    if is_installed(expect)? {
        return Ok(expect.to_string());
    }
    let staging = stage(expect)?;
    let tar = fs::File::open(file).with_context(|| format!("cannot open {}", file.display()))?;
    extract(tar, &staging.0)?;
    set_modes(&staging.0)?;
    verify(&staging.0, expect)?;
    publish(staging, expect)?;
    Ok(expect.to_string())
}

/// Extract regular files and folders only. A link could point out of the
/// folder, and a path with `..` or a root could write out of it. The sizes in
/// the headers are summed before a file is written.
fn extract(source: impl io::Read, dest: &Path) -> Result<()> {
    let (mut count, mut bytes) = (0_usize, 0_u64);
    for entry in Archive::new(source).entries()? {
        let mut entry = entry?;
        count += 1;
        if count > MAX_ENTRIES {
            bail!("the tar has too many entries (over {MAX_ENTRIES})");
        }
        bytes = bytes.saturating_add(entry.header().size()?);
        if bytes > MAX_BYTES {
            bail!("the tar is too large (over {MAX_BYTES} bytes)");
        }
        let path = entry.path()?.into_owned();
        let shown = path.display();
        let kind = entry.header().entry_type();
        if kind == EntryType::XGlobalHeader || graph::is_apple_double(&path) {
            continue;
        }
        if !matches!(kind, EntryType::Regular | EntryType::Directory) {
            bail!("`{shown}` is a link or a special file, and a graph has none");
        }
        let leaves = path.components().any(|part| {
            matches!(
                part,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        });
        if leaves || !entry.unpack_in(dest)? {
            bail!("`{shown}` is outside the graph folder");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_names_both_hashes_when_they_differ() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join("index.md"), "x\n").unwrap();
        let real = graph::hash(tmp.path()).unwrap();

        verify(tmp.path(), &real).unwrap();
        let error = verify(tmp.path(), &"0".repeat(64)).unwrap_err().to_string();

        assert!(error.contains("hash mismatch"), "{error}");
        assert!(error.contains(&real), "{error}");
    }
}
