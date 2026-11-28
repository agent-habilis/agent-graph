//! `graph install`: put a graph folder at `~/.agent-graph/graphs/<hash>`.
//!
//! The graph is staged in a folder of its own and moved into place only when
//! it is complete and its hash is right, so a failed install leaves nothing.

use std::path::{Path, PathBuf};
use std::{fs, io};

use agent_graph_core::graph;
use anyhow::{Context, Result, bail};
use tar::{Archive, EntryType};

/// A folder that is removed when it goes out of scope, unless it was moved.
struct Staging(PathBuf);

impl Drop for Staging {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// Where the graphs of this user are installed.
fn graphs_dir() -> Result<PathBuf> {
    let home = std::env::var_os("HOME").context("`$HOME` is not set")?;
    Ok(PathBuf::from(home).join(".agent-graph/graphs"))
}

/// A new empty folder next to the final one.
fn stage(base: &Path, hash: &str) -> Result<Staging> {
    let path = base.join(format!(".{hash}.tmp-{}", std::process::id()));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).with_context(|| format!("cannot create {}", path.display()))?;
    Ok(Staging(path))
}

/// Move the staged graph to `<base>/<hash>`. A graph that is already there
/// has the same hash, so it has the same content, and the staged one goes.
fn publish(staging: Staging, base: &Path, hash: &str) -> Result<()> {
    let dest = base.join(hash);
    match fs::rename(&staging.0, &dest) {
        Ok(()) => {
            std::mem::forget(staging);
            Ok(())
        }
        Err(_) if dest.is_dir() => Ok(()),
        Err(error) => Err(error).with_context(|| format!("cannot create {}", dest.display())),
    }
}

/// Copy a graph folder to `~/.agent-graph/graphs/<hash>`. Returns the hash.
pub(crate) fn from_folder(folder: &Path) -> Result<String> {
    let hash = graph::hash(folder).with_context(|| format!("cannot read {}", folder.display()))?;
    let base = graphs_dir()?;
    fs::create_dir_all(&base).with_context(|| format!("cannot create {}", base.display()))?;
    let staging = stage(&base, &hash)?;
    copy_dir(folder, &staging.0)?;
    publish(staging, &base, &hash)?;
    Ok(hash)
}

fn copy_dir(from: &Path, to: &Path) -> Result<()> {
    for entry in fs::read_dir(from).with_context(|| format!("cannot read {}", from.display()))? {
        let path = entry?.path();
        let target = to.join(path.file_name().expect("a folder entry has a name"));
        if path.is_dir() {
            fs::create_dir_all(&target)?;
            copy_dir(&path, &target)?;
        } else {
            fs::copy(&path, &target).with_context(|| format!("cannot copy {}", path.display()))?;
        }
    }
    Ok(())
}

/// Extract a tar file of a graph folder to `~/.agent-graph/graphs/<expect>`.
/// The folder stays only if its hash is `expect`. Returns the hash.
pub(crate) fn from_tar(file: &Path, expect: &str) -> Result<String> {
    let base = graphs_dir()?;
    fs::create_dir_all(&base).with_context(|| format!("cannot create {}", base.display()))?;
    let staging = stage(&base, expect)?;
    let tar = fs::File::open(file).with_context(|| format!("cannot open {}", file.display()))?;
    extract(tar, &staging.0)?;
    let hash = graph::hash(&staging.0)?;
    if hash != expect {
        bail!("hash mismatch: expected {expect}, got {hash}");
    }
    publish(staging, &base, expect)?;
    Ok(hash)
}

/// Extract regular files and folders only. A link could point out of the
/// folder, and a path with `..` could write out of it.
fn extract(source: impl io::Read, dest: &Path) -> Result<()> {
    for entry in Archive::new(source).entries()? {
        let mut entry = entry?;
        let path = entry.path()?.display().to_string();
        let kind = entry.header().entry_type();
        if kind == EntryType::XGlobalHeader {
            continue;
        }
        if !matches!(kind, EntryType::Regular | EntryType::Directory) {
            bail!("`{path}` is a link or a special file, and a graph has none");
        }
        if !entry.unpack_in(dest)? {
            bail!("`{path}` is outside the graph folder");
        }
    }
    Ok(())
}
