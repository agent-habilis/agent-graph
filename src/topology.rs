use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::{env, fs};

use anyhow::{Context, Result, anyhow, bail};
use serde_json::Value;

use crate::graph::{self, Class, Graph};
use crate::node;

/// The gossip documents, as printed by `agent-gossip state get`, `meta get`,
/// and `peers`.
#[derive(Debug)]
pub(crate) struct Gossip {
    state: Value,
    meta: Value,
    roster: Value,
}

impl Gossip {
    pub(crate) fn read(state: &Path, meta: &Path, roster: &Path) -> Result<Self> {
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
            state: read(state)?,
            meta: read(meta)?,
            roster: read(roster)?,
        })
    }

    fn meta_peer(&self, nick: &str) -> &Value {
        &self.meta["document"]["peers"][nick]
    }

    fn roster_peer(&self, nick: &str) -> Option<&Value> {
        self.roster["peers"]
            .as_array()?
            .iter()
            .find(|peer| peer["nickname"] == nick)
    }
}

/// Draw the pod `instance`, or the pod of `me`, as a Mermaid flowchart. Also
/// return one warning per problem, sorted.
pub(crate) fn draw(
    graphs: &BTreeMap<String, PathBuf>,
    gossip: &Gossip,
    instance: Option<&str>,
    me: &str,
) -> Result<(String, Vec<String>)> {
    let instance = match instance {
        Some(instance) => instance,
        None if gossip.meta_peer(me).is_null() => bail!("`{me}` is not in the gossip meta"),
        None => gossip.meta_peer(me)["graph"]
            .as_str()
            .ok_or_else(|| anyhow!("`{me}` is not in a pod"))?,
    };
    let pod = &gossip.state["document"]["graphs"][instance];
    let hash = pod["hash"]
        .as_str()
        .ok_or_else(|| anyhow!("pod `{instance}` not found in the gossip state"))?;
    let dir = find_graph(graphs, instance, hash)?;
    let graph =
        graph::load(&dir).map_err(|reason| anyhow!("graph {hash} is not valid: {reason}"))?;
    Ok(render(&dir, &graph, gossip, instance, pod, me))
}

/// The graph folder with this hash: the graph of the same name that the cwd
/// sees, else a graph that a peer received in `~/.agent-graph/<hash>`.
fn find_graph(graphs: &BTreeMap<String, PathBuf>, instance: &str, hash: &str) -> Result<PathBuf> {
    let name = instance.split('@').next().unwrap_or(instance);
    let received =
        env::var_os("HOME").map(|home| PathBuf::from(home).join(".agent-graph").join(hash));
    let local = graphs.get(name);
    if let Some(dir) = local
        .into_iter()
        .chain(&received)
        .find(|dir| graph::hash(dir).is_ok_and(|found| found == hash))
    {
        return Ok(dir.clone());
    }
    match local.map(|dir| graph::hash(dir)) {
        Some(Ok(found)) => bail!(
            "graph `{name}` has changed since the pod started: its hash is {}, the pod has {}",
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
    pod: &Value,
    me: &str,
) -> (String, Vec<String>) {
    let bootstrapper = pod["bootstrapper"].as_str().unwrap_or("");
    let placed = placements(pod);
    let mut warnings = Vec::new();
    let mut vertices = String::new();
    let mut ids: BTreeMap<&str, Vec<String>> = BTreeMap::new();
    let (mut filled, mut total) = (0, 0);
    warnings.extend(unknown_nodes(graph, pod));
    for vertex in &graph.vertices {
        if vertex.subgraph.is_none() {
            ids.insert(&vertex.id, vec![mermaid_id(&vertex.id)]);
            continue;
        }
        let entry = &pod["nodes"][&vertex.id];
        let node = node::load(&graph::node_dir(dir, vertex)).ok();
        let count = entry["count"]
            .as_u64()
            .or(node.as_ref().map(|node| u64::from(node.count)))
            .unwrap_or(1);
        let preferred = node.and_then(|node| node.model);
        let peers = placed.get(vertex.id.as_str()).cloned().unwrap_or_default();
        filled += peers.len() as u64;
        total += count;
        warnings.extend(count_problem(&vertex.label, peers.len() as u64, count));
        let (open_shape, close_shape) = match vertex.class {
            Some(Class::Public) => ('(', ')'),
            _ => ('[', ']'),
        };
        let mut peer_ids = Vec::new();
        for (nick, times) in peers {
            let meta = gossip.meta_peer(nick);
            let mut problems = problems(
                gossip,
                nick,
                meta,
                instance,
                &vertex.id,
                preferred.as_deref(),
                me,
            );
            if times > 1 {
                problems.push(format!("listed {times} times"));
            }
            let nodes = placed
                .values()
                .filter(|listed| listed.iter().any(|(peer, _)| *peer == nick))
                .count();
            if nodes > 1 {
                problems.push(format!("in {nodes} nodes"));
            }
            let label = label(
                &vertex.label,
                nick,
                meta,
                nick == me,
                nick == bootstrapper,
                !problems.is_empty(),
            );
            warnings.extend(
                problems
                    .into_iter()
                    .map(|problem| format!("{} · {nick} · {problem}", vertex.label)),
            );
            let id = format!(
                "{}__{}__{}",
                mermaid_id(instance),
                mermaid_id(&vertex.id),
                mermaid_id(nick)
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
    warnings.extend(unlisted(gossip, instance, &placed));

    let mut out = header(graph, instance, pod, filled, total);
    out += &vertices;
    out += "  end\n";
    out += &edges(graph, &ids);
    let mut warnings: Vec<String> = warnings.iter().map(|warning| one_line(warning)).collect();
    warnings.sort();
    (out, warnings)
}

/// The first lines: the direction, the external vertices, and the pod box.
fn header(graph: &Graph, instance: &str, pod: &Value, filled: u64, total: u64) -> String {
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
    let short = hash_prefix(pod["hash"].as_str().unwrap_or(""));
    let _ = writeln!(
        out,
        "  subgraph \"{} · {filled}/{total} · {short}\"",
        clean(instance)
    );
    out
}

/// A warning for each state node that the graph does not have.
fn unknown_nodes(graph: &Graph, pod: &Value) -> Vec<String> {
    pod["nodes"]
        .as_object()
        .into_iter()
        .flat_map(|nodes| nodes.keys())
        .filter(|id| {
            !graph
                .vertices
                .iter()
                .any(|vertex| vertex.subgraph.is_some() && vertex.id == **id)
        })
        .map(|id| format!("{id} · not in the graph"))
        .collect()
}

/// The peers of each state node, once each, in list order, with the number of
/// times that the list has them.
fn placements(pod: &Value) -> BTreeMap<&str, Vec<(&str, usize)>> {
    let mut placed: BTreeMap<&str, Vec<(&str, usize)>> = BTreeMap::new();
    for (id, entry) in pod["nodes"].as_object().into_iter().flatten() {
        let peers = placed.entry(id).or_default();
        for nick in entry["peers"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
        {
            match peers.iter_mut().find(|(peer, _)| *peer == nick) {
                Some((_, times)) => *times += 1,
                None => peers.push((nick, 1)),
            }
        }
    }
    placed
}

fn count_problem(node: &str, peers: u64, count: u64) -> Option<String> {
    let places = |number: u64| if number == 1 { "place" } else { "places" };
    match peers.cmp(&count) {
        Ordering::Less => {
            let open = count - peers;
            Some(format!("{node} · {open} open {}", places(open)))
        }
        Ordering::Greater => Some(format!(
            "{node} · {peers} peers for {count} {}",
            places(count)
        )),
        Ordering::Equal => None,
    }
}

fn label(node: &str, nick: &str, meta: &Value, me: bool, bootstrapper: bool, warn: bool) -> String {
    let mut label = format!("{node} · {nick}");
    if me {
        label += " ← you";
    }
    if bootstrapper {
        label += " ★";
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

/// A warning for each peer whose meta names this pod and a node, but that no
/// state node lists.
fn unlisted(
    gossip: &Gossip,
    instance: &str,
    placed: &BTreeMap<&str, Vec<(&str, usize)>>,
) -> Vec<String> {
    let mut warnings = Vec::new();
    for (nick, meta) in gossip.meta["document"]["peers"]
        .as_object()
        .into_iter()
        .flatten()
    {
        let Some(node) = meta["node"].as_str() else {
            continue;
        };
        let listed = placed.values().flatten().any(|(peer, _)| peer == nick);
        if meta["graph"] == instance && !listed {
            warnings.push(format!("{node} · {nick} · in the meta, not in the state"));
        }
    }
    warnings
}

/// Each graph edge, from every peer of one node to every peer of the other.
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

fn problems(
    gossip: &Gossip,
    nick: &str,
    meta: &Value,
    instance: &str,
    node: &str,
    preferred: Option<&str>,
    me: &str,
) -> Vec<String> {
    let mut problems = Vec::new();
    if let (Some(preferred), Some(model)) = (preferred, meta["model"].as_str())
        && !model.to_lowercase().contains(&preferred.to_lowercase())
    {
        problems.push(format!("model {model} does not match {preferred}"));
    }
    match gossip.roster_peer(nick) {
        Some(peer) if peer["quiet"] == true => problems.push("quiet".to_string()),
        None if nick != me => problems.push("gone from the roster".to_string()),
        _ => {}
    }
    for (key, want) in [("graph", instance), ("node", node)] {
        let have = meta[key].as_str().unwrap_or("none");
        if have != want {
            problems.push(format!("meta {key} {have} does not match {want}"));
        }
    }
    problems
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
pub(crate) fn render_mermaid(source: &str) -> Result<String> {
    agent_habilis_render::mermaid::mermaid(source)
}
