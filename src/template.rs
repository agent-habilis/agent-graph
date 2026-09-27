use std::collections::{BTreeMap, HashMap};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use std::{fs, io};

use regex::Regex;
use sha2::{Digest, Sha256};

use crate::{frontmatter, markdown, role};

/// The class that marks the lead role of a team. Any other class is only
/// style.
const LEAD: &str = "lead";

/// A vertex in the mermaid block: a role, or an external party.
#[derive(Debug)]
pub(crate) struct Vertex {
    pub(crate) id: String,
    /// The label, which names a folder in `<template>/roles/`.
    pub(crate) label: String,
    pub(crate) lead: bool,
    /// The subgraph (team) that holds the role. `None` for an external party.
    pub(crate) subgraph: Option<String>,
    line: usize,
}

#[derive(Debug)]
pub(crate) struct Handoff {
    pub(crate) name: Option<String>,
    pub(crate) from: String,
    pub(crate) to: String,
    /// `-->`, `<-->`, or `---`.
    pub(crate) arrow: String,
    /// The `|label|` text, without quotes.
    pub(crate) label: Option<String>,
    line: usize,
}

#[derive(Debug)]
pub(crate) struct Template {
    pub(crate) description: String,
    pub(crate) scope: String,
    pub(crate) title: Option<String>,
    pub(crate) icon: Option<String>,
    pub(crate) vertices: Vec<Vertex>,
    pub(crate) handoffs: Vec<Handoff>,
    content: String,
    /// Problems found while reading, as (line, reason). `lint` reports them.
    problems: Vec<(usize, String)>,
}

static HANDOFF: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^([A-Za-z_][\w-]*)\s+(?:([a-z0-9-]+)@)?(<-->|-->|---)\s*(?:\|([^|]*)\|)?\s*([A-Za-z_][\w-]*)$")
        .expect("valid regex")
});
static VERTEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^([A-Za-z_][\w-]*)([\[({>].*[\])}])?(?::::(\w+))?$").expect("valid regex")
});

/// Read `<dir>/index.md` of a team template. Frontmatter errors fail the
/// load; mermaid problems are kept for `lint`, so that `roles` can still read
/// a template with a missing handoff section.
pub(crate) fn load(dir: &Path) -> Result<Template, String> {
    let content =
        fs::read_to_string(dir.join("index.md")).map_err(|err| format!("index.md: {err}"))?;
    let frontmatter =
        frontmatter::parse(&content).ok_or("index.md has no frontmatter with a `type:` key")?;
    match frontmatter.get("type") {
        Some("TeamTemplate") => {}
        other => {
            return Err(format!(
                "type is `{}`, must be `TeamTemplate`",
                other.unwrap_or("")
            ));
        }
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
    let title = frontmatter.get("title").map(str::to_string);
    let icon = frontmatter.get("icon").map(str::to_string);
    let mut template = Template {
        description,
        scope,
        title,
        icon,
        vertices: Vec::new(),
        handoffs: Vec::new(),
        content: String::new(),
        problems: Vec::new(),
    };
    parse_mermaid(&content, &mut template);
    template.content = content;
    Ok(template)
}

fn parse_mermaid(content: &str, template: &mut Template) {
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
        template
            .problems
            .push((1, "no ```mermaid block".to_string()));
        return;
    };
    if fences.len() > 1 {
        template
            .problems
            .push((fences[1], "more than one ```mermaid block".to_string()));
    }
    let mut subgraph: Option<String> = None;
    let mut header = false;
    let mut leads: Vec<String> = Vec::new();
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
                template
                    .problems
                    .push((number, format!("unknown line `{line}`")));
                continue;
            };
            if name == LEAD {
                leads.extend(ids.split(',').map(str::to_string));
            }
        } else if let Some(caps) = HANDOFF.captures(line) {
            template.handoffs.push(Handoff {
                name: caps.get(2).map(|name| name.as_str().to_string()),
                from: caps[1].to_string(),
                to: caps[5].to_string(),
                arrow: caps[3].to_string(),
                label: caps
                    .get(4)
                    .map(|label| label.as_str().trim().trim_matches('"').to_string())
                    .filter(|label| !label.is_empty()),
                line: number,
            });
        } else if let Some(caps) = VERTEX.captures(line) {
            let id = caps[1].to_string();
            let label = caps.get(2).map_or(id.clone(), |shape| {
                shape
                    .as_str()
                    .trim_matches(|char: char| "[](){}>\"".contains(char))
                    .to_string()
            });
            template.vertices.push(Vertex {
                id,
                label,
                lead: caps.get(3).is_some_and(|class| class.as_str() == LEAD),
                subgraph: subgraph.clone(),
                line: number,
            });
        } else {
            template
                .problems
                .push((number, format!("unknown line `{line}`")));
        }
    }
    for id in leads {
        if let Some(vertex) = template.vertices.iter_mut().find(|vertex| vertex.id == id) {
            vertex.lead = true;
        }
    }
}

/// The SHA-256 of the template folder at `dir`, as hex: the id of the
/// template between peers. It covers the relative path and the content of each file,
/// in path order, and no timestamps, so a copy of the folder has the same
/// hash.
pub(crate) fn hash(dir: &Path) -> io::Result<String> {
    let mut files = Vec::new();
    collect_files(dir, &mut files)?;
    files.sort();
    let mut hasher = Sha256::new();
    for file in files {
        let content = fs::read(&file)?;
        let relative = file.strip_prefix(dir).unwrap_or(&file);
        hasher.update(relative.to_string_lossy().as_bytes());
        hasher.update(b"\0");
        hasher.update((content.len() as u64).to_le_bytes());
        hasher.update(&content);
    }
    let mut hex = String::with_capacity(64);
    for byte in hasher.finalize() {
        let _ = write!(hex, "{byte:02x}");
    }
    Ok(hex)
}

fn collect_files(dir: &Path, files: &mut Vec<PathBuf>) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            collect_files(&path, files)?;
        } else {
            files.push(path);
        }
    }
    Ok(())
}

/// The role folder of a vertex in the template at `dir`.
pub(crate) fn role_dir(dir: &Path, vertex: &Vertex) -> PathBuf {
    dir.join("roles").join(&vertex.label)
}

/// Check the template against the template rules. Each error is (line,
/// reason).
pub(crate) fn lint(dir: &Path, template: &Template) -> Vec<(usize, String)> {
    let mut errors = template.problems.clone();
    let by_id: HashMap<&str, &Vertex> = template
        .vertices
        .iter()
        .map(|vertex| (vertex.id.as_str(), vertex))
        .collect();

    errors.extend(role_errors(dir, template));

    let mut names: Vec<&str> = Vec::new();
    for handoff in &template.handoffs {
        let ends = [handoff.from.as_str(), handoff.to.as_str()];
        let Some(from) = by_id.get(ends[0]) else {
            errors.push((
                handoff.line,
                format!("handoff to unknown role `{}`", ends[0]),
            ));
            continue;
        };
        let Some(to) = by_id.get(ends[1]) else {
            errors.push((
                handoff.line,
                format!("handoff to unknown role `{}`", ends[1]),
            ));
            continue;
        };
        if from.subgraph != to.subgraph {
            for role in [from, to] {
                if role.subgraph.is_some() && !role.lead {
                    errors.push((
                        handoff.line,
                        format!(
                            "handoff crosses the team boundary at role `{}`, which is not the lead",
                            role.id
                        ),
                    ));
                }
            }
        }
        match &handoff.name {
            None => errors.push((handoff.line, "handoff has no name".to_string())),
            Some(name) if names.contains(&name.as_str()) => errors.push((
                handoff.line,
                format!("handoff name `{name}` is used more than once"),
            )),
            Some(name) => names.push(name),
        }
    }

    let sections = handoff_sections(&template.content);
    for handoff in &template.handoffs {
        if let Some(name) = &handoff.name
            && !sections.iter().any(|(_, section, _)| section == name)
        {
            errors.push((
                handoff.line,
                format!("handoff `{name}` has no `### {name}` section"),
            ));
        }
    }
    for (line, name, has_text) in &sections {
        if !names.contains(&name.as_str()) {
            errors.push((*line, format!("section `{name}` names no handoff")));
        } else if !has_text {
            errors.push((*line, format!("section `{name}` is empty")));
        }
    }

    if let Err(error) = markdown::check_boundaries(&template.content) {
        errors.push(error);
    }
    errors.sort();
    errors.dedup();
    errors
}

/// An error for each role folder that is missing or not valid, and for each
/// team that does not have exactly one lead.
fn role_errors(dir: &Path, template: &Template) -> Vec<(usize, String)> {
    let mut errors = Vec::new();
    let mut leads: BTreeMap<&str, Vec<&Vertex>> = BTreeMap::new();
    for role in template
        .vertices
        .iter()
        .filter(|role| role.subgraph.is_some())
    {
        let subgraph = role.subgraph.as_deref().unwrap_or("");
        let entry = leads.entry(subgraph).or_default();
        if role.lead {
            entry.push(role);
        }
        let folder = role_dir(dir, role);
        if !folder.is_dir() {
            errors.push((role.line, format!("role `{}` not found", role.label)));
        } else if let Err(reason) = role::load(&folder) {
            errors.push((
                role.line,
                format!("role `{}` is not valid: {reason}", role.label),
            ));
        }
    }
    for (subgraph, roles) in &leads {
        let line = template
            .vertices
            .iter()
            .find(|role| role.subgraph.as_deref() == Some(subgraph))
            .map_or(1, |role| role.line);
        match roles.len() {
            0 => errors.push((line, format!("team `{subgraph}` has no lead"))),
            1 => {}
            _ => errors.push((
                roles[1].line,
                format!("team `{subgraph}` has more than one lead"),
            )),
        }
    }
    errors
}

/// The `### <name>` sections under `## Handoffs`, as (line, name, has text).
fn handoff_sections(content: &str) -> Vec<(usize, String, bool)> {
    let Some(section) = markdown::section(content, "## Handoffs") else {
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
