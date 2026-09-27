use std::path::PathBuf;
use std::{fs, io};

use anyhow::Result;
use serde_json::{Map, Value};

use crate::role::Role;
use crate::template::Template;

const DIR: &str = "/tmp/agent-graph";

/// Write `/tmp/agent-graph/<pid>.json`: the template and role of the agent
/// with that pid, for the statusline. The keys stay `graph` and `node`,
/// because agent-statusline reads them. The write is atomic, so a statusline
/// never reads half a file.
pub(crate) fn write(pid: u32, template: (&str, &Template), role: (&str, &Role)) -> Result<()> {
    let mut state = Map::new();
    state.insert("pid".into(), pid.into());
    let template_name = template.1.title.as_deref().unwrap_or(template.0);
    insert(
        &mut state,
        "graph",
        template_name,
        template.1.icon.as_deref(),
    );
    insert(&mut state, "node", role.0, role.1.icon.as_deref());
    let dir = PathBuf::from(DIR);
    fs::create_dir_all(&dir)?;
    let path = dir.join(format!("{pid}.json"));
    let tmp = dir.join(format!("{pid}.json.tmp"));
    fs::write(&tmp, format!("{}\n", Value::Object(state)))?;
    fs::rename(&tmp, &path)?;
    Ok(())
}

/// Remove `/tmp/agent-graph/<pid>.json`. A missing file is not an error: the
/// agent has no role either way.
pub(crate) fn remove(pid: u32) -> Result<()> {
    match fs::remove_file(PathBuf::from(DIR).join(format!("{pid}.json"))) {
        Err(err) if err.kind() != io::ErrorKind::NotFound => Err(err.into()),
        _ => Ok(()),
    }
}

fn insert(state: &mut Map<String, Value>, key: &str, name: &str, icon: Option<&str>) {
    state.insert(key.into(), name.into());
    if let Some(icon) = icon {
        state.insert(format!("{key}_icon"), icon.into());
    }
}
