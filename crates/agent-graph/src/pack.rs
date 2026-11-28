//! `graph pack`: write a graph folder as a tar file that `graph install --tar`
//! accepts.
//!
//! The tar holds the files that the graph hash covers, as regular files, in
//! path order, with the same header for each: mode 0644, owner 0, time 0. It
//! has no extended attributes and no `._` entries, whatever the file system
//! holds, so the same graph gives the same bytes on each machine.

use std::path::Path;

use agent_graph_core::graph;
use anyhow::{Context, Result, bail};

use crate::install::{MAX_BYTES, MAX_ENTRIES};
use crate::output;

pub(crate) fn pack(dir: &Path, output: &Path) -> Result<()> {
    let files = graph::files(dir).with_context(|| format!("cannot read {}", dir.display()))?;
    if files.len() > MAX_ENTRIES {
        bail!("the graph has too many files (over {MAX_ENTRIES})");
    }
    let mut builder = tar::Builder::new(Vec::new());
    let mut total = 0_u64;
    for file in files {
        let data =
            std::fs::read(&file).with_context(|| format!("cannot read {}", file.display()))?;
        total = total.saturating_add(data.len() as u64);
        if total > MAX_BYTES {
            bail!("the graph is too large (over {MAX_BYTES} bytes)");
        }
        let relative = file.strip_prefix(dir).unwrap_or(&file);
        let mut header = tar::Header::new_gnu();
        header.set_entry_type(tar::EntryType::Regular);
        header.set_size(data.len() as u64);
        header.set_mode(0o644);
        header.set_uid(0);
        header.set_gid(0);
        header.set_mtime(0);
        builder
            .append_data(&mut header, relative, data.as_slice())
            .with_context(|| format!("cannot pack {}", file.display()))?;
    }
    let bytes = builder.into_inner().context("cannot finish the tar")?;
    output::write_new(output, &bytes)
}
