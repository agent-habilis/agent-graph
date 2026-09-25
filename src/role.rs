use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::{frontmatter, markdown};

#[derive(Debug)]
pub(crate) struct Role {
    pub(crate) description: String,
    pub(crate) tags: Vec<String>,
}

/// Bundle folders in `.agent-roles/<kind>/` from `start` up to `/`, by name.
/// A nearer folder shadows a farther one with the same name, valid or not.
pub(crate) fn discover(start: &Path, kind: &str) -> BTreeMap<String, PathBuf> {
    let mut bundles = BTreeMap::new();
    for level in start.ancestors() {
        // A missing or unreadable level has no bundles for us; it must not
        // stop the walk.
        let Ok(entries) = fs::read_dir(level.join(".agent-roles").join(kind)) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            if let Some(name) = path.file_name().and_then(|name| name.to_str()) {
                bundles.entry(name.to_string()).or_insert(path);
            }
        }
    }
    bundles
}

/// Read and validate `<dir>/index.md`. The error is a reason for the user.
pub(crate) fn load(dir: &Path) -> Result<Role, String> {
    let content =
        fs::read_to_string(dir.join("index.md")).map_err(|err| format!("index.md: {err}"))?;
    let frontmatter =
        frontmatter::parse(&content).ok_or("index.md has no frontmatter with a `type:` key")?;
    match frontmatter.get("type") {
        Some("Role") => {}
        other => return Err(format!("type is `{}`, must be `Role`", other.unwrap_or(""))),
    }
    let description = frontmatter
        .get("description")
        .filter(|description| !description.is_empty())
        .ok_or("no description")?
        .to_string();
    let tags = match frontmatter.get("tags") {
        None => Vec::new(),
        Some(value) => frontmatter::inline_list(value)
            .ok_or("tags must be an inline list, for example `tags: [a, b]`")?,
    };
    markdown::check_boundaries(&content)
        .map_err(|(line, reason)| format!("index.md:{line}: {reason}"))?;
    Ok(Role { description, tags })
}
