use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use regex::Regex;

use crate::{frontmatter, markdown};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Class {
    Public,
    Private,
}

impl Class {
    fn parse(name: &str) -> Option<Self> {
        match name {
            "public" => Some(Self::Public),
            "private" => Some(Self::Private),
            _ => None,
        }
    }

    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Public => "public",
            Self::Private => "private",
        }
    }
}

#[derive(Debug)]
pub(crate) struct Node {
    pub(crate) id: String,
    /// The node label, which names its role.
    pub(crate) label: String,
    pub(crate) class: Option<Class>,
    /// The subgraph that holds the node. `None` for an external node.
    pub(crate) subgraph: Option<String>,
    line: usize,
}

#[derive(Debug)]
struct Edge {
    name: Option<String>,
    from: String,
    to: String,
    line: usize,
}

#[derive(Debug)]
pub(crate) struct Pod {
    pub(crate) description: String,
    pub(crate) scope: String,
    pub(crate) nodes: Vec<Node>,
    edges: Vec<Edge>,
    content: String,
    /// Problems found while reading, as (line, reason). `lint` reports them.
    problems: Vec<(usize, String)>,
}

static EDGE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^([A-Za-z_][\w-]*)\s+(?:([a-z0-9-]+)@)?(<-->|-->|---)\s*(?:\|[^|]*\|)?\s*([A-Za-z_][\w-]*)$")
        .expect("valid regex")
});
static NODE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^([A-Za-z_][\w-]*)([\[({>].*[\])}])?(?::::(\w+))?$").expect("valid regex")
});

/// Read `<dir>/index.md` of a pod. Frontmatter errors fail the load; graph
/// problems are kept for `lint`, so that `nodes` can still read a pod with
/// a missing edge section.
pub(crate) fn load(dir: &Path) -> Result<Pod, String> {
    let content =
        fs::read_to_string(dir.join("index.md")).map_err(|err| format!("index.md: {err}"))?;
    let frontmatter =
        frontmatter::parse(&content).ok_or("index.md has no frontmatter with a `type:` key")?;
    match frontmatter.get("type") {
        Some("Pod") => {}
        other => return Err(format!("type is `{}`, must be `Pod`", other.unwrap_or(""))),
    }
    let description = frontmatter
        .get("description")
        .filter(|description| !description.is_empty())
        .ok_or("no description")?
        .to_string();
    let scope = frontmatter.get("scope").unwrap_or("").to_string();
    if !matches!(scope.as_str(), "project" | "branch") {
        return Err(format!("scope is `{scope}`, must be `project` or `branch`"));
    }
    let mut pod = Pod {
        description,
        scope,
        nodes: Vec::new(),
        edges: Vec::new(),
        content: String::new(),
        problems: Vec::new(),
    };
    parse_graph(&content, &mut pod);
    pod.content = content;
    Ok(pod)
}

fn parse_graph(content: &str, pod: &mut Pod) {
    let mut fences = Vec::new();
    let mut in_fence = false;
    for (index, line) in content.lines().enumerate() {
        if markdown::is_fence(line) {
            if !in_fence && line.trim() == "```mermaid" {
                fences.push(index + 1);
            }
            in_fence = !in_fence;
        }
    }
    let Some(&start) = fences.first() else {
        pod.problems.push((1, "no ```mermaid block".to_string()));
        return;
    };
    if fences.len() > 1 {
        pod.problems
            .push((fences[1], "more than one ```mermaid block".to_string()));
    }
    let mut subgraph: Option<String> = None;
    let mut header = false;
    let mut classes: Vec<(String, Class)> = Vec::new();
    for (index, raw) in content.lines().enumerate().skip(start) {
        let number = index + 1;
        let line = raw.trim();
        if markdown::is_fence(line) {
            break;
        }
        if line.is_empty() || line.starts_with("%%") || line.starts_with("classDef ") {
            continue;
        }
        if !header {
            header = true;
            if line.starts_with("flowchart ") || line.starts_with("graph ") {
                continue;
            }
        }
        if let Some(rest) = line.strip_prefix("subgraph ") {
            let id = rest
                .split(|char: char| char.is_whitespace() || char == '[')
                .next()
                .unwrap_or("");
            subgraph = Some(id.to_string());
        } else if line == "end" {
            subgraph = None;
        } else if let Some(rest) = line.strip_prefix("class ") {
            let mut parts = rest.split_whitespace();
            let (Some(ids), Some(name)) = (parts.next(), parts.next()) else {
                pod.problems
                    .push((number, format!("unknown line `{line}`")));
                continue;
            };
            if let Some(class) = Class::parse(name) {
                classes.extend(ids.split(',').map(|id| (id.to_string(), class)));
            }
        } else if let Some(caps) = EDGE.captures(line) {
            let edge = |from: &str, to: &str| Edge {
                name: caps.get(2).map(|name| name.as_str().to_string()),
                from: from.to_string(),
                to: to.to_string(),
                line: number,
            };
            pod.edges.push(edge(&caps[1], &caps[4]));
        } else if let Some(caps) = NODE.captures(line) {
            let id = caps[1].to_string();
            let label = caps.get(2).map_or(id.clone(), |shape| {
                shape
                    .as_str()
                    .trim_matches(|char: char| "[](){}>\"".contains(char))
                    .to_string()
            });
            pod.nodes.push(Node {
                id,
                label,
                class: caps.get(3).and_then(|class| Class::parse(class.as_str())),
                subgraph: subgraph.clone(),
                line: number,
            });
        } else {
            pod.problems
                .push((number, format!("unknown line `{line}`")));
        }
    }
    for (id, class) in classes {
        if let Some(node) = pod.nodes.iter_mut().find(|node| node.id == id) {
            node.class = Some(class);
        }
    }
}

/// The role name of a node in the pod at `dir`: `<pod>/<label>` if the pod
/// folder holds that role, else `<label>` if `roles` has it.
pub(crate) fn role_of(
    pod_name: &str,
    dir: &Path,
    node: &Node,
    roles: &BTreeMap<String, PathBuf>,
) -> Option<String> {
    let local = format!("{pod_name}/{}", node.label);
    if dir.join("roles").join(&node.label).is_dir() {
        Some(local)
    } else {
        roles.contains_key(&node.label).then(|| node.label.clone())
    }
}

/// Check the pod against the pod rules. Each error is (line, reason).
pub(crate) fn lint(
    pod_name: &str,
    dir: &Path,
    pod: &Pod,
    roles: &BTreeMap<String, PathBuf>,
) -> Vec<(usize, String)> {
    let mut errors = pod.problems.clone();
    let by_id: HashMap<&str, &Node> = pod
        .nodes
        .iter()
        .map(|node| (node.id.as_str(), node))
        .collect();

    let mut publics: BTreeMap<&str, Vec<&Node>> = BTreeMap::new();
    for node in pod.nodes.iter().filter(|node| node.subgraph.is_some()) {
        let subgraph = node.subgraph.as_deref().unwrap_or("");
        let entry = publics.entry(subgraph).or_default();
        match node.class {
            Some(Class::Public) => entry.push(node),
            Some(Class::Private) => {}
            None => errors.push((
                node.line,
                format!("node `{}` is not public or private", node.id),
            )),
        }
        if role_of(pod_name, dir, node, roles).is_none() {
            errors.push((node.line, format!("role `{}` not found", node.label)));
        }
    }
    for (subgraph, nodes) in &publics {
        let line = pod
            .nodes
            .iter()
            .find(|node| node.subgraph.as_deref() == Some(subgraph))
            .map_or(1, |node| node.line);
        match nodes.len() {
            0 => errors.push((line, format!("pod `{subgraph}` has no public node"))),
            1 => {}
            _ => errors.push((
                nodes[1].line,
                format!("pod `{subgraph}` has more than one public node"),
            )),
        }
    }

    let mut names: Vec<&str> = Vec::new();
    for edge in &pod.edges {
        let ends = [edge.from.as_str(), edge.to.as_str()];
        let Some(from) = by_id.get(ends[0]) else {
            errors.push((edge.line, format!("edge to unknown node `{}`", ends[0])));
            continue;
        };
        let Some(to) = by_id.get(ends[1]) else {
            errors.push((edge.line, format!("edge to unknown node `{}`", ends[1])));
            continue;
        };
        if from.subgraph != to.subgraph {
            for node in [from, to] {
                if node.subgraph.is_some() && node.class != Some(Class::Public) {
                    errors.push((
                        edge.line,
                        format!(
                            "edge crosses the pod boundary at private node `{}`",
                            node.id
                        ),
                    ));
                }
            }
        }
        match &edge.name {
            None => errors.push((edge.line, "edge has no name".to_string())),
            Some(name) if names.contains(&name.as_str()) => errors.push((
                edge.line,
                format!("edge name `{name}` is used more than once"),
            )),
            Some(name) => names.push(name),
        }
    }

    let sections = edge_sections(&pod.content);
    for edge in &pod.edges {
        if let Some(name) = &edge.name
            && !sections.iter().any(|(_, section, _)| section == name)
        {
            errors.push((
                edge.line,
                format!("edge `{name}` has no `### {name}` section"),
            ));
        }
    }
    for (line, name, has_text) in &sections {
        if !names.contains(&name.as_str()) {
            errors.push((*line, format!("section `{name}` names no edge")));
        } else if !has_text {
            errors.push((*line, format!("section `{name}` is empty")));
        }
    }

    if let Err(error) = markdown::check_boundaries(&pod.content) {
        errors.push(error);
    }
    errors.sort();
    errors.dedup();
    errors
}

/// The `### <name>` sections under `## Edges`, as (line, name, has text).
fn edge_sections(content: &str) -> Vec<(usize, String, bool)> {
    let Some(section) = markdown::section(content, "## Edges") else {
        return Vec::new();
    };
    let mut sections: Vec<(usize, String, bool)> = Vec::new();
    for (number, line) in section.lines {
        if let Some(name) = line.strip_prefix("### ") {
            sections.push((number, name.trim().to_string(), false));
        } else if !line.trim().is_empty()
            && let Some(last) = sections.last_mut()
        {
            last.2 = true;
        }
    }
    sections
}
