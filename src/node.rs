use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::{frontmatter, markdown};

#[derive(Debug)]
pub(crate) struct Node {
    pub(crate) description: String,
    pub(crate) tags: Vec<String>,
    pub(crate) icon: Option<String>,
    /// How many agents the node needs.
    pub(crate) count: u32,
    /// The model the node prefers, matched against a peer's model.
    pub(crate) model: Option<String>,
}

/// Graph folders in `.agent-graph/` from `start` up to `/`, by name. A nearer
/// folder shadows a farther one with the same name, valid or not.
pub(crate) fn discover_graphs(start: &Path) -> BTreeMap<String, PathBuf> {
    let mut graphs = BTreeMap::new();
    for level in start.ancestors() {
        // A missing or unreadable level has no graphs for us; it must not
        // stop the walk.
        let Ok(entries) = fs::read_dir(level.join(".agent-graph")) else {
            continue;
        };
        for (name, path) in subfolders(entries) {
            graphs.entry(name).or_insert(path);
        }
    }
    graphs
}

/// Node folders from `start` up to `/`, named `<graph>/<node>`. A node comes
/// only from the nearest graph of that name, so a graph is shadowed as one
/// unit.
pub(crate) fn discover_nodes(start: &Path) -> BTreeMap<String, PathBuf> {
    let mut nodes = BTreeMap::new();
    for (graph, dir) in discover_graphs(start) {
        let Ok(entries) = fs::read_dir(dir.join("nodes")) else {
            continue;
        };
        for (name, path) in subfolders(entries) {
            nodes.insert(format!("{graph}/{name}"), path);
        }
    }
    nodes
}

fn subfolders(entries: fs::ReadDir) -> impl Iterator<Item = (String, PathBuf)> {
    entries.flatten().filter_map(|entry| {
        let path = entry.path();
        let name = path.file_name()?.to_str()?.to_string();
        path.is_dir().then_some((name, path))
    })
}

/// Read and validate `<dir>/index.md`. The error is a reason for the user.
pub(crate) fn load(dir: &Path) -> Result<Node, String> {
    let content =
        fs::read_to_string(dir.join("index.md")).map_err(|err| format!("index.md: {err}"))?;
    let frontmatter =
        frontmatter::parse(&content).ok_or("index.md has no frontmatter with a `type:` key")?;
    match frontmatter.get("type") {
        Some("Node") => {}
        other => return Err(format!("type is `{}`, must be `Node`", other.unwrap_or(""))),
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
    let count = match frontmatter.get("count") {
        None => 1,
        Some(value) => value
            .parse()
            .ok()
            .filter(|count| *count >= 1)
            .ok_or_else(|| format!("count is `{value}`, must be a whole number of 1 or more"))?,
    };
    markdown::check_boundaries(&content)
        .map_err(|(line, reason)| format!("index.md:{line}: {reason}"))?;
    let icon = frontmatter.get("icon").map(str::to_string);
    let model = frontmatter.get("model").map(str::to_string);
    Ok(Node {
        description,
        tags,
        icon,
        count,
        model,
    })
}
