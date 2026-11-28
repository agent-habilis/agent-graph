use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::{env, fs};

use agent_graph_core::graph::{self, Graph};
use agent_graph_core::node;
use anyhow::{Context, Result, anyhow, bail};
use serde_json::Value;

/// The gossip documents, as printed by `agent-gossip meta get` and `peers`.
#[derive(Debug)]
pub struct Gossip {
    meta: Value,
    roster: Value,
}

impl Gossip {
    /// Read the meta and roster JSON files.
    ///
    /// # Errors
    ///
    /// Fails when a file cannot be read or parsed, or holds an
    /// `agent-gossip` error.
    pub fn read(meta: &Path, roster: &Path) -> Result<Self> {
        let read = |path: &Path| -> Result<Value> {
            let text =
                fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
            let value: Value =
                serde_json::from_str(&text).with_context(|| format!("parse {}", path.display()))?;
            if value["ok"] == false {
                bail!(
                    "{}: agent-gossip failed: {}",
                    path.display(),
                    value["error"]
                );
            }
            Ok(value)
        };
        Ok(Self {
            meta: read(meta)?,
            roster: read(roster)?,
        })
    }

    fn entry(&self, key: &str) -> &Value {
        &self.meta["document"][key]
    }

    /// The meta entries of the graph, each with its session key: each live
    /// peer whose `instance` is `instance`, sorted by nickname. A peer in
    /// `absent` left or died, so its entry is history.
    fn graph_entries(&self, instance: &str) -> Vec<(&str, &Value)> {
        let absent: Vec<&str> = self.meta["absent"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .collect();
        let mut entries: Vec<(&str, &Value)> = self.meta["document"]
            .as_object()
            .into_iter()
            .flatten()
            .filter(|(key, entry)| entry["instance"] == instance && !absent.contains(&key.as_str()))
            .map(|(key, entry)| (key.as_str(), entry))
            .collect();
        entries.sort_by_key(|&(key, entry)| (nick(key, entry), key));
        entries
    }

    fn roster_peer(&self, key: &str) -> Option<&Value> {
        self.roster["peers"]
            .as_array()?
            .iter()
            .find(|peer| peer["pubkey"] == key)
    }
}

/// The nickname of a meta entry: a field of the entry, not its key. Two
/// peers can share a nickname, so never join an entry by it.
fn nick<'a>(key: &'a str, entry: &'a Value) -> &'a str {
    entry["nickname"].as_str().unwrap_or(key)
}

/// The peers of each node, each with its session key and its meta entry.
type Placed<'a> = BTreeMap<&'a str, Vec<(&'a str, &'a Value)>>;

/// Draw the graph `instance`, or the graph of `me`, as a Mermaid flowchart.
/// `me` is the session key of this agent.
/// Also return one warning per problem, sorted.
///
/// # Errors
///
/// Fails when the graph has no peers, its hash is not clear, or no local
/// graph has that hash.
pub fn draw(
    graphs: &BTreeMap<String, PathBuf>,
    gossip: &Gossip,
    instance: Option<&str>,
    me: &str,
) -> Result<(String, Vec<String>)> {
    let instance = match instance {
        Some(instance) => instance,
        None if gossip.entry(me).is_null() => bail!("`{me}` is not in the gossip meta"),
        None => gossip.entry(me)["instance"]
            .as_str()
            .ok_or_else(|| anyhow!("`{me}` is not in a graph"))?,
    };
    let entries = gossip.graph_entries(instance);
    if entries.is_empty() {
        bail!("graph `{instance}` has no peers");
    }
    let hash = graph_hash(&entries, me, instance)?;
    let name = entries
        .iter()
        .find(|(_, entry)| entry["hash"] == hash)
        .and_then(|(_, entry)| entry["graph"].as_str());
    let dir = find_graph(graphs, name, hash)?;
    let graph =
        graph::load(&dir).map_err(|reason| anyhow!("graph {hash} is not valid: {reason}"))?;
    Ok(render(&dir, &graph, gossip, instance, hash, &entries, me))
}

/// The hash of the running graph: the hash of `me` if `me` is in the graph,
/// else the one hash that its entries share. Two graphs with one name are an
/// error.
fn graph_hash<'a>(entries: &[(&str, &'a Value)], me: &str, instance: &str) -> Result<&'a str> {
    if let Some(hash) = entries
        .iter()
        .find(|(key, _)| *key == me)
        .and_then(|(_, entry)| entry["hash"].as_str())
    {
        return Ok(hash);
    }
    let mut hashes: Vec<&str> = entries
        .iter()
        .filter_map(|(_, entry)| entry["hash"].as_str())
        .collect();
    hashes.sort_unstable();
    hashes.dedup();
    match hashes.as_slice() {
        [hash] => Ok(hash),
        [] => bail!("graph `{instance}` has no hash in the gossip meta"),
        _ => bail!(
            "graph `{instance}` has {} hashes: {}",
            hashes.len(),
            hashes
                .iter()
                .map(|hash| hash_prefix(hash))
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}

/// The graph folder with this hash: a graph that the cwd sees, else a
/// graph that a peer received in `~/.agent-graph/graphs/<hash>`.
fn find_graph(
    graphs: &BTreeMap<String, PathBuf>,
    name: Option<&str>,
    hash: &str,
) -> Result<PathBuf> {
    let received =
        env::var_os("HOME").map(|home| PathBuf::from(home).join(".agent-graph/graphs").join(hash));
    if let Some(dir) = graphs
        .values()
        .chain(&received)
        .find(|dir| graph::hash(dir).is_ok_and(|found| found == hash))
    {
        return Ok(dir.clone());
    }
    match name.and_then(|name| Some((name, graph::hash(graphs.get(name)?)))) {
        Some((name, Ok(found))) => bail!(
            "graph `{name}` has changed since the instance started: its hash is {}, the instance has {}",
            hash_prefix(&found),
            hash_prefix(hash)
        ),
        _ => bail!("graph {hash} not found"),
    }
}

fn render(
    dir: &Path,
    graph: &Graph,
    gossip: &Gossip,
    instance: &str,
    hash: &str,
    entries: &[(&str, &Value)],
    me: &str,
) -> (String, Vec<String>) {
    let mut warnings = entry_problems(instance, hash, entries);
    let peers: Vec<(&str, &Value)> = entries
        .iter()
        .filter(|(_, entry)| {
            entry["hash"] == hash && entry["graph"].is_string() && entry["node"].is_string()
        })
        .copied()
        .collect();
    let mut placed: Placed<'_> = BTreeMap::new();
    for &(key, entry) in &peers {
        let node = entry["node"].as_str().unwrap_or_default();
        placed.entry(node).or_default().push((key, entry));
    }
    warnings.extend(unknown_nodes(graph, &placed));
    let mut vertices = String::new();
    let mut ids: BTreeMap<&str, Vec<String>> = BTreeMap::new();
    let mut drawn = 0;
    for vertex in &graph.vertices {
        if vertex.subgraph.is_none() {
            ids.insert(&vertex.id, vec![mermaid_id(&vertex.id)]);
            continue;
        }
        let node = node::load(&graph::node_dir(dir, vertex)).ok();
        let count = node
            .as_ref()
            .map_or(1, |node| usize::try_from(node.count).unwrap_or(usize::MAX));
        let preferred = node.and_then(|node| node.model);
        let members = placed.get(vertex.id.as_str()).cloned().unwrap_or_default();
        drawn += members.len();
        if members.len() < count {
            let open = count - members.len();
            let noun = if open == 1 { "place" } else { "places" };
            warnings.push(format!("{} · {open} open {noun}", vertex.label));
        }
        let (open_shape, close_shape) = if vertex.lead {
            if members.len() > 1 {
                warnings.push(format!(
                    "{} · {} peers in the lead node",
                    vertex.label,
                    members.len()
                ));
            }
            ('(', ')')
        } else {
            ('[', ']')
        };
        let mut peer_ids = Vec::new();
        for (key, meta) in members {
            let nick = nick(key, meta);
            let mut problems = problems(gossip, key, meta, preferred.as_deref(), me);
            if let Some(inviter) = meta["invited_by"].as_str()
                && !peers
                    .iter()
                    .any(|&(peer, entry)| self::nick(peer, entry) == inviter)
            {
                problems.push(format!("invited_by {inviter} is not a peer"));
            }
            let label = label(&vertex.label, nick, meta, key == me, !problems.is_empty());
            warnings.extend(
                problems
                    .into_iter()
                    .map(|problem| format!("{} · {nick} · {problem}", vertex.label)),
            );
            let id = format!(
                "{}__{}__{}",
                mermaid_id(instance),
                mermaid_id(&vertex.id),
                mermaid_id(key.get(..8).unwrap_or(key))
            );
            let _ = writeln!(
                vertices,
                "    {id}{open_shape}{}{close_shape}",
                clean(&label)
            );
            peer_ids.push(id);
        }
        ids.insert(&vertex.id, peer_ids);
    }

    let mut out = header(graph, instance, hash, drawn);
    out += &vertices;
    out += "  end\n";
    out += &edges(graph, &ids);
    let mut warnings: Vec<String> = warnings.iter().map(|warning| one_line(warning)).collect();
    warnings.sort();
    (out, warnings)
}

/// A warning for entries with another hash, for each entry that misses a key,
/// and for more than one founder: an entry without `invited_by` started the
/// graph.
fn entry_problems(instance: &str, hash: &str, entries: &[(&str, &Value)]) -> Vec<String> {
    let mut warnings = Vec::new();
    let other: Vec<&str> = entries
        .iter()
        .filter(|(_, entry)| entry["hash"].is_string() && entry["hash"] != hash)
        .map(|&(key, entry)| nick(key, entry))
        .collect();
    if !other.is_empty() {
        let peers = if other.len() == 1 { "peer" } else { "peers" };
        warnings.push(format!(
            "{instance} · {} {peers} with another hash · {}",
            other.len(),
            other.join(" ")
        ));
    }
    let same: Vec<&(&str, &Value)> = entries
        .iter()
        .filter(|(_, entry)| !entry["hash"].is_string() || entry["hash"] == hash)
        .collect();
    for &&(key, entry) in &same {
        if let Some(missing) = ["hash", "graph", "node"]
            .into_iter()
            .find(|field| !entry[*field].is_string())
        {
            let nick = nick(key, entry);
            warnings.push(format!("{instance} · {nick} · meta has no {missing}"));
        }
    }
    let founders: Vec<&str> = same
        .iter()
        .filter(|(_, entry)| entry["invited_by"].is_null())
        .map(|&&(key, entry)| nick(key, entry))
        .collect();
    if founders.len() > 1 {
        warnings.push(format!(
            "{instance} · {} founders · {}",
            founders.len(),
            founders.join(" ")
        ));
    }
    warnings
}

/// The first lines: the direction, the external vertices, and the graph box.
fn header(graph: &Graph, instance: &str, hash: &str, drawn: usize) -> String {
    let mut out = String::from("flowchart TD\n");
    for vertex in graph
        .vertices
        .iter()
        .filter(|vertex| vertex.subgraph.is_none())
    {
        let _ = writeln!(
            out,
            "  {}[{}]",
            mermaid_id(&vertex.id),
            clean(&vertex.label)
        );
    }
    let peers = if drawn == 1 { "peer" } else { "peers" };
    let _ = writeln!(
        out,
        "  subgraph \"{} · {drawn} {peers} · {}\"",
        clean(instance),
        hash_prefix(hash)
    );
    out
}

/// A warning for each peer in a node that the graph does not have.
fn unknown_nodes(graph: &Graph, placed: &Placed<'_>) -> Vec<String> {
    placed
        .iter()
        .filter(|(id, _)| {
            !graph
                .vertices
                .iter()
                .any(|vertex| vertex.subgraph.is_some() && vertex.id == **id)
        })
        .flat_map(|(id, members)| {
            members
                .iter()
                .map(move |&(key, entry)| format!("{id} · {} · not in the graph", nick(key, entry)))
        })
        .collect()
}

fn label(node: &str, nick: &str, meta: &Value, me: bool, warn: bool) -> String {
    let mut label = format!("{node} · {nick}");
    if me {
        label += " ← you";
    }
    for key in ["model", "status"] {
        if let Some(value) = meta[key].as_str() {
            let _ = write!(label, " · {value}");
        }
    }
    if warn {
        label += " ⚠";
    }
    label
}

fn problems(
    gossip: &Gossip,
    key: &str,
    meta: &Value,
    preferred: Option<&str>,
    me: &str,
) -> Vec<String> {
    let mut problems = Vec::new();
    if let (Some(preferred), Some(model)) = (preferred, meta["model"].as_str())
        && !model.to_lowercase().contains(&preferred.to_lowercase())
    {
        problems.push(format!("model {model} does not match {preferred}"));
    }
    // A peer without agent-graph cannot hash the graph folder. It joins
    // with the node context that the inviter sent, and says so.
    if meta["verified"] == false {
        problems.push("graph not verified".to_string());
    }
    match gossip.roster_peer(key) {
        Some(peer) if peer["quiet"] == true => problems.push("quiet".to_string()),
        None if key != me => problems.push("gone from the roster".to_string()),
        _ => {}
    }
    problems
}

/// Each edge of the graph, from every peer of one node to every
/// peer of the other.
fn edges(graph: &Graph, ids: &BTreeMap<&str, Vec<String>>) -> String {
    let mut out = String::new();
    for edge in &graph.edges {
        let (Some(from), Some(to)) = (ids.get(edge.from.as_str()), ids.get(edge.to.as_str()))
        else {
            continue;
        };
        // figurehead, the Mermaid renderer of `ahre`, does not parse `<-->`.
        let arrow = if edge.arrow == "<-->" {
            "---"
        } else {
            &edge.arrow
        };
        let label = edge
            .label
            .as_ref()
            .or(edge.name.as_ref())
            .map(|label| format!("|{}|", clean(label)))
            .unwrap_or_default();
        for from_id in from {
            for to_id in to {
                let _ = writeln!(out, "  {from_id} {arrow}{label} {to_id}");
            }
        }
    }
    out
}

/// A Mermaid id: each character that is not alphanumeric becomes `_`, so
/// that a `-` is not read as part of an arrow.
fn mermaid_id(text: &str) -> String {
    text.chars()
        .map(|char| {
            if char.is_ascii_alphanumeric() {
                char
            } else {
                '_'
            }
        })
        .collect()
}

/// One line: each control character, such as a line break, becomes a space.
fn one_line(text: &str) -> String {
    text.chars()
        .map(|char| if char.is_control() { ' ' } else { char })
        .collect()
}

/// A one-line label without the characters that end a Mermaid label.
fn clean(text: &str) -> String {
    one_line(text)
        .chars()
        .filter(|char| !"[](){}|\"".contains(*char))
        .collect()
}

fn hash_prefix(hash: &str) -> &str {
    hash.get(..6).unwrap_or(hash)
}

/// Draw Mermaid source as terminal text, with the vendored render.
///
/// # Errors
///
/// Fails when the source is not valid Mermaid.
pub fn render_mermaid(source: &str) -> Result<String> {
    agent_habilis_render::mermaid::mermaid(source)
}
