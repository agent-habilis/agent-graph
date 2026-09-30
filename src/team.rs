use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::{env, fs};

use anyhow::{Context, Result, anyhow, bail};
use serde_json::Value;

use crate::role;
use crate::template::{self, Template};

/// The gossip documents, as printed by `agent-gossip meta get` and `peers`.
#[derive(Debug)]
pub(crate) struct Gossip {
    meta: Value,
    roster: Value,
}

impl Gossip {
    pub(crate) fn read(meta: &Path, roster: &Path) -> Result<Self> {
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

    fn meta_peer(&self, nick: &str) -> &Value {
        &self.meta["document"]["peers"][nick]
    }

    /// The meta entries of the team: each live member whose `team` is
    /// `instance`. A peer in `absent` left or died, so its entry is history.
    fn team_entries(&self, instance: &str) -> Vec<(&str, &Value)> {
        let absent: Vec<&str> = self.meta["absent"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .collect();
        self.meta["document"]["peers"]
            .as_object()
            .into_iter()
            .flatten()
            .filter(|(nick, entry)| entry["team"] == instance && !absent.contains(&nick.as_str()))
            .map(|(nick, entry)| (nick.as_str(), entry))
            .collect()
    }

    fn roster_peer(&self, nick: &str) -> Option<&Value> {
        self.roster["peers"]
            .as_array()?
            .iter()
            .find(|peer| peer["nickname"] == nick)
    }
}

/// Draw the team `instance`, or the team of `me`, as a Mermaid flowchart.
/// Also return one warning per problem, sorted.
pub(crate) fn draw(
    templates: &BTreeMap<String, PathBuf>,
    gossip: &Gossip,
    instance: Option<&str>,
    me: &str,
) -> Result<(String, Vec<String>)> {
    let instance = match instance {
        Some(instance) => instance,
        None if gossip.meta_peer(me).is_null() => bail!("`{me}` is not in the gossip meta"),
        None => gossip.meta_peer(me)["team"]
            .as_str()
            .ok_or_else(|| anyhow!("`{me}` is not in a team"))?,
    };
    let entries = gossip.team_entries(instance);
    if entries.is_empty() {
        bail!("team `{instance}` has no members");
    }
    let hash = team_hash(&entries, me, instance)?;
    let name = entries
        .iter()
        .find(|(_, entry)| entry["hash"] == hash)
        .and_then(|(_, entry)| entry["template"].as_str());
    let dir = find_template(templates, name, hash)?;
    let template =
        template::load(&dir).map_err(|reason| anyhow!("template {hash} is not valid: {reason}"))?;
    Ok(render(
        &dir, &template, gossip, instance, hash, &entries, me,
    ))
}

/// The template hash of the team: the hash of `me` if `me` is in the team,
/// else the one hash that its entries share. Two teams with one name are an
/// error.
fn team_hash<'a>(entries: &[(&str, &'a Value)], me: &str, instance: &str) -> Result<&'a str> {
    if let Some(hash) = entries
        .iter()
        .find(|(nick, _)| *nick == me)
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
        [] => bail!("team `{instance}` has no hash in the gossip meta"),
        _ => bail!(
            "team `{instance}` has {} hashes: {}",
            hashes.len(),
            hashes
                .iter()
                .map(|hash| hash_prefix(hash))
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}

/// The template folder with this hash: a template that the cwd sees, else a
/// template that a member received in `~/.agent-graph/<hash>`.
fn find_template(
    templates: &BTreeMap<String, PathBuf>,
    name: Option<&str>,
    hash: &str,
) -> Result<PathBuf> {
    let received =
        env::var_os("HOME").map(|home| PathBuf::from(home).join(".agent-graph").join(hash));
    if let Some(dir) = templates
        .values()
        .chain(&received)
        .find(|dir| template::hash(dir).is_ok_and(|found| found == hash))
    {
        return Ok(dir.clone());
    }
    match name.and_then(|name| Some((name, template::hash(templates.get(name)?)))) {
        Some((name, Ok(found))) => bail!(
            "template `{name}` has changed since the team started: its hash is {}, the team has {}",
            hash_prefix(&found),
            hash_prefix(hash)
        ),
        _ => bail!("template {hash} not found"),
    }
}

fn render(
    dir: &Path,
    template: &Template,
    gossip: &Gossip,
    instance: &str,
    hash: &str,
    entries: &[(&str, &Value)],
    me: &str,
) -> (String, Vec<String>) {
    let mut warnings = entry_problems(instance, hash, entries);
    let members: Vec<(&str, &Value)> = entries
        .iter()
        .filter(|(_, entry)| {
            entry["hash"] == hash && entry["template"].is_string() && entry["role"].is_string()
        })
        .copied()
        .collect();
    let mut placed: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for (nick, entry) in &members {
        let role = entry["role"].as_str().unwrap_or_default();
        placed.entry(role).or_default().push(nick);
    }
    warnings.extend(unknown_roles(template, &placed));
    let mut vertices = String::new();
    let mut ids: BTreeMap<&str, Vec<String>> = BTreeMap::new();
    let mut drawn = 0;
    for vertex in &template.vertices {
        if vertex.subgraph.is_none() {
            ids.insert(&vertex.id, vec![mermaid_id(&vertex.id)]);
            continue;
        }
        let role = role::load(&template::role_dir(dir, vertex)).ok();
        let count = role
            .as_ref()
            .map_or(1, |role| usize::try_from(role.count).unwrap_or(usize::MAX));
        let preferred = role.and_then(|role| role.model);
        let nicks = placed.get(vertex.id.as_str()).cloned().unwrap_or_default();
        drawn += nicks.len();
        if nicks.len() < count {
            let open = count - nicks.len();
            let noun = if open == 1 { "place" } else { "places" };
            warnings.push(format!("{} · {open} open {noun}", vertex.label));
        }
        let (open_shape, close_shape) = if vertex.lead {
            if nicks.len() > 1 {
                warnings.push(format!(
                    "{} · {} members in the lead role",
                    vertex.label,
                    nicks.len()
                ));
            }
            ('(', ')')
        } else {
            ('[', ']')
        };
        let mut member_ids = Vec::new();
        for nick in nicks {
            let meta = gossip.meta_peer(nick);
            let mut problems = problems(gossip, nick, meta, preferred.as_deref(), me);
            if let Some(inviter) = meta["invited_by"].as_str()
                && !members.iter().any(|(member, _)| *member == inviter)
            {
                problems.push(format!("invited_by {inviter} is not a member"));
            }
            let label = label(&vertex.label, nick, meta, nick == me, !problems.is_empty());
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
            member_ids.push(id);
        }
        ids.insert(&vertex.id, member_ids);
    }

    let mut out = header(template, instance, hash, drawn);
    out += &vertices;
    out += "  end\n";
    out += &handoffs(template, &ids);
    let mut warnings: Vec<String> = warnings.iter().map(|warning| one_line(warning)).collect();
    warnings.sort();
    (out, warnings)
}

/// A warning for entries with another hash, for each entry that misses a key,
/// and for more than one founder: an entry without `invited_by` started the
/// team.
fn entry_problems(instance: &str, hash: &str, entries: &[(&str, &Value)]) -> Vec<String> {
    let mut warnings = Vec::new();
    let other: Vec<&str> = entries
        .iter()
        .filter(|(_, entry)| entry["hash"].is_string() && entry["hash"] != hash)
        .map(|(nick, _)| *nick)
        .collect();
    if !other.is_empty() {
        let members = if other.len() == 1 {
            "member"
        } else {
            "members"
        };
        warnings.push(format!(
            "{instance} · {} {members} with another hash · {}",
            other.len(),
            other.join(" ")
        ));
    }
    let same: Vec<&(&str, &Value)> = entries
        .iter()
        .filter(|(_, entry)| !entry["hash"].is_string() || entry["hash"] == hash)
        .collect();
    for (nick, entry) in &same {
        if let Some(key) = ["hash", "template", "role"]
            .into_iter()
            .find(|key| !entry[*key].is_string())
        {
            warnings.push(format!("{instance} · {nick} · meta has no {key}"));
        }
    }
    let founders: Vec<&str> = same
        .iter()
        .filter(|(_, entry)| entry["invited_by"].is_null())
        .map(|(nick, _)| *nick)
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

/// The first lines: the direction, the external vertices, and the team box.
fn header(template: &Template, instance: &str, hash: &str, drawn: usize) -> String {
    let mut out = String::from("flowchart TD\n");
    for vertex in template
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
    let members = if drawn == 1 { "member" } else { "members" };
    let _ = writeln!(
        out,
        "  subgraph \"{} · {drawn} {members} · {}\"",
        clean(instance),
        hash_prefix(hash)
    );
    out
}

/// A warning for each member in a role that the template does not have.
fn unknown_roles(template: &Template, placed: &BTreeMap<&str, Vec<&str>>) -> Vec<String> {
    placed
        .iter()
        .filter(|(id, _)| {
            !template
                .vertices
                .iter()
                .any(|vertex| vertex.subgraph.is_some() && vertex.id == **id)
        })
        .flat_map(|(id, nicks)| {
            nicks
                .iter()
                .map(move |nick| format!("{id} · {nick} · not in the template"))
        })
        .collect()
}

fn label(role: &str, nick: &str, meta: &Value, me: bool, warn: bool) -> String {
    let mut label = format!("{role} · {nick}");
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
    nick: &str,
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
    // A peer without agent-graph cannot hash the template folder. It joins
    // with the role context that the inviter sent, and says so.
    if meta["verified"] == false {
        problems.push("template not verified".to_string());
    }
    match gossip.roster_peer(nick) {
        Some(peer) if peer["quiet"] == true => problems.push("quiet".to_string()),
        None if nick != me => problems.push("gone from the roster".to_string()),
        _ => {}
    }
    problems
}

/// Each handoff of the template, from every member of one role to every
/// member of the other.
fn handoffs(template: &Template, ids: &BTreeMap<&str, Vec<String>>) -> String {
    let mut out = String::new();
    for handoff in &template.handoffs {
        let (Some(from), Some(to)) = (ids.get(handoff.from.as_str()), ids.get(handoff.to.as_str()))
        else {
            continue;
        };
        // figurehead, the Mermaid renderer of `ahre`, does not parse `<-->`.
        let arrow = if handoff.arrow == "<-->" {
            "---"
        } else {
            &handoff.arrow
        };
        let label = handoff
            .label
            .as_ref()
            .or(handoff.name.as_ref())
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
pub(crate) fn render_mermaid(source: &str) -> Result<String> {
    agent_habilis_render::mermaid::mermaid(source)
}
