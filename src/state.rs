use std::fs;
use std::path::PathBuf;

use anyhow::Result;
use serde_json::{Map, Value};

use crate::graph::Graph;
use crate::node::Node;

/// Write `/tmp/agent-graph/<pid>.json`: the graph and node of the agent with
/// that pid, for the statusline. The write is atomic, so a statusline never
/// reads half a file.
pub(crate) fn write(pid: u32, graph: (&str, &Graph), node: (&str, &Node)) -> Result<()> {
    let mut state = Map::new();
    state.insert("pid".into(), pid.into());
    insert(&mut state, "graph", graph.0, graph.1.icon.as_deref());
    insert(&mut state, "node", node.0, node.1.icon.as_deref());
    let dir = PathBuf::from("/tmp/agent-graph");
    fs::create_dir_all(&dir)?;
    let path = dir.join(format!("{pid}.json"));
    let tmp = dir.join(format!("{pid}.json.tmp"));
    fs::write(&tmp, format!("{}\n", Value::Object(state)))?;
    fs::rename(&tmp, &path)?;
    Ok(())
}

fn insert(state: &mut Map<String, Value>, key: &str, name: &str, icon: Option<&str>) {
    state.insert(key.into(), name.into());
    if let Some(icon) = icon {
        state.insert(format!("{key}_icon"), icon.into());
    }
}
