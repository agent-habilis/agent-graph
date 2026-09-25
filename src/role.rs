use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::{frontmatter, markdown};

#[derive(Debug)]
pub(crate) struct Role {
    pub(crate) description: String,
    pub(crate) tags: Vec<String>,
    pub(crate) icon: Option<String>,
}

#[derive(Debug)]
pub(crate) struct Pod {
    pub(crate) icon: Option<String>,
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
        for (name, path) in subfolders(entries) {
            bundles.entry(name).or_insert(path);
        }
    }
    bundles
}

/// Role folders from `start` up to `/`, by name. A role inside a pod is
/// named `<pod>/<role>`, and comes only from the nearest pod of that name,
/// so a pod is shadowed as one unit.
pub(crate) fn discover_roles(start: &Path) -> BTreeMap<String, PathBuf> {
    let mut roles = discover(start, "roles");
    for (pod, dir) in discover(start, "pods") {
        let Ok(entries) = fs::read_dir(dir.join("roles")) else {
            continue;
        };
        for (name, path) in subfolders(entries) {
            roles.insert(format!("{pod}/{name}"), path);
        }
    }
    roles
}

fn subfolders(entries: fs::ReadDir) -> impl Iterator<Item = (String, PathBuf)> {
    entries.flatten().filter_map(|entry| {
        let path = entry.path();
        let name = path.file_name()?.to_str()?.to_string();
        path.is_dir().then_some((name, path))
    })
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
    let icon = frontmatter.get("icon").map(str::to_string);
    Ok(Role {
        description,
        tags,
        icon,
    })
}

/// Read `<dir>/index.md` of a pod. Only checks what `get` needs.
pub(crate) fn load_pod(dir: &Path) -> Result<Pod, String> {
    let content =
        fs::read_to_string(dir.join("index.md")).map_err(|err| format!("index.md: {err}"))?;
    let frontmatter =
        frontmatter::parse(&content).ok_or("index.md has no frontmatter with a `type:` key")?;
    match frontmatter.get("type") {
        Some("Pod") => {}
        other => return Err(format!("type is `{}`, must be `Pod`", other.unwrap_or(""))),
    }
    let icon = frontmatter.get("icon").map(str::to_string);
    Ok(Pod { icon })
}
