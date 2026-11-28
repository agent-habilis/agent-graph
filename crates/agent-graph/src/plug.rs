//! `agent-graph plug` / `unplug`: install or remove the graph skills in each
//! agent. The skills are rendered by `build.rs` and embedded, so the binary
//! needs no repo checkout. Ported from agent-gossip `src/cli/plug.rs`.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use include_dir::{Dir, include_dir};

static SKILLS: Dir<'_> = include_dir!("$OUT_DIR/skills");
/// The same skills, with each slash command in the namespaced form that agy
/// expands: `/agent-graph:graph-up`.
static SKILLS_AGY: Dir<'_> = include_dir!("$OUT_DIR/skills-agy");

// Ties this module to the rendered skills, so a changed skill recompiles the
// `include_dir!` embed above, which is not tracked on its own.
const _: &str = env!("AGENT_GRAPH_SKILLS_FINGERPRINT");

#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub(crate) enum Agent {
    /// Claude Code: skills in `~/.claude/skills`.
    #[value(name = "claude-code", alias = "claude")]
    ClaudeCode,
    /// pi: skills in `~/.pi/agent/skills`.
    Pi,
    /// Codex: skills in `~/.codex/skills`.
    Codex,
    /// Cursor: skills in `~/.cursor/skills`.
    Cursor,
    /// opencode: skills in `~/.config/opencode/skills`.
    #[value(name = "opencode")]
    OpenCode,
    /// Antigravity CLI: a plugin in `~/.gemini/config/plugins/agent-graph`.
    #[value(name = "antigravity", alias = "agy")]
    Antigravity,
}

impl Agent {
    const ALL: [Agent; 6] = [
        Agent::ClaudeCode,
        Agent::Pi,
        Agent::Codex,
        Agent::Cursor,
        Agent::OpenCode,
        Agent::Antigravity,
    ];

    fn label(self) -> &'static str {
        match self {
            Agent::ClaudeCode => "claude-code",
            Agent::Pi => "pi",
            Agent::Codex => "codex",
            Agent::Cursor => "cursor",
            Agent::OpenCode => "opencode",
            Agent::Antigravity => "antigravity",
        }
    }

    /// Its presence is the detection signal: `plug` never installs into an
    /// agent that is not on this machine.
    fn agent_dir(self, home: &Path) -> PathBuf {
        home.join(match self {
            Agent::ClaudeCode => ".claude",
            Agent::Pi => ".pi",
            Agent::Codex => ".codex",
            Agent::Cursor => ".cursor",
            Agent::OpenCode => ".config/opencode",
            Agent::Antigravity => ".gemini/antigravity-cli",
        })
    }

    /// The plugin form lets the tool install and unplug as one unit, with the
    /// same layout as agent-gossip, which also ships a `hooks.json`. agy
    /// also loads plain skills from `~/.gemini/config/skills`.
    fn plugin_dir(self, home: &Path) -> Option<PathBuf> {
        (self == Agent::Antigravity).then(|| home.join(".gemini/config/plugins/agent-graph"))
    }

    fn skills(self) -> &'static Dir<'static> {
        match self {
            Agent::Antigravity => &SKILLS_AGY,
            Agent::ClaudeCode | Agent::Pi | Agent::Codex | Agent::Cursor | Agent::OpenCode => {
                &SKILLS
            }
        }
    }

    fn skills_dir(self, home: &Path) -> PathBuf {
        match self {
            Agent::Pi => home.join(".pi/agent/skills"),
            Agent::Antigravity => home.join(".gemini/config/plugins/agent-graph/skills"),
            Agent::ClaudeCode | Agent::Codex | Agent::Cursor | Agent::OpenCode => {
                self.agent_dir(home).join("skills")
            }
        }
    }

    fn detected(self, home: &Path) -> bool {
        self.agent_dir(home).exists()
    }

    fn installed(self, home: &Path) -> bool {
        let skills = self.skills_dir(home);
        owned_dirs(&skills)
            .iter()
            .any(|dir| dir.symlink_metadata().is_ok())
            || skills.join(MANIFEST).symlink_metadata().is_ok()
            || self
                .plugin_dir(home)
                .is_some_and(|plugin| manifest_is_ours(&plugin))
    }
}

/// Write the skills into each agent (default: the detected ones) and into
/// each `--path`. Print one line per target.
pub(crate) fn plug(agents: &[Agent], paths: &[PathBuf]) -> Result<Vec<String>> {
    let home = home()?;
    let mut lines = Vec::new();
    for agent in selected(agents, paths, |agent| agent.detected(&home)) {
        if agent.detected(&home) {
            let dir = agent.skills_dir(&home);
            install(&dir, agent.skills())?;
            let kept = match agent.plugin_dir(&home) {
                Some(plugin) => write_manifest(&plugin)?,
                None => None,
            };
            lines.push(format!("installed\t{}\t{}", agent.label(), dir.display()));
            if let Some((path, reason)) = kept {
                lines.push(format!(
                    "kept\t{}\t{}\tplugin.json is {reason}, left untouched",
                    agent.label(),
                    path.display()
                ));
            }
        } else {
            lines.push(format!("skipped\t{}\tnot detected", agent.label()));
        }
    }
    for path in paths {
        install(path, &SKILLS)?;
        lines.push(format!("installed\tpath\t{}", path.display()));
    }
    Ok(lines)
}

/// Remove the skills from each agent (default: the ones that have them) and
/// from each `--path`. Other folders stay.
pub(crate) fn unplug(agents: &[Agent], paths: &[PathBuf]) -> Result<Vec<String>> {
    let home = home()?;
    let mut lines = Vec::new();
    for agent in selected(agents, paths, |agent| agent.installed(&home)) {
        let dir = agent.skills_dir(&home);
        let mut removed = remove(&dir)?;
        if let Some(plugin) = agent.plugin_dir(&home) {
            removed |= remove_manifest(&plugin)?;
        }
        let state = if removed { "removed" } else { "skipped" };
        lines.push(format!("{state}\t{}\t{}", agent.label(), dir.display()));
    }
    for path in paths {
        let state = if remove(path)? { "removed" } else { "skipped" };
        lines.push(format!("{state}\tpath\t{}", path.display()));
    }
    Ok(lines)
}

/// Explicit `--agent` flags win. Only `--path` means no agent. Else the
/// agents that `default` selects.
fn selected(agents: &[Agent], paths: &[PathBuf], default: impl Fn(Agent) -> bool) -> Vec<Agent> {
    if !agents.is_empty() {
        let mut unique = Vec::new();
        for &agent in agents {
            if !unique.contains(&agent) {
                unique.push(agent);
            }
        }
        return unique;
    }
    if !paths.is_empty() {
        return Vec::new();
    }
    Agent::ALL
        .into_iter()
        .filter(|&agent| default(agent))
        .collect()
}

fn home() -> Result<PathBuf> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .context("`$HOME` is not set")
}

/// Lists the skills that the last `plug` wrote into a root, so that a later
/// binary also removes a skill that it renamed or dropped.
const MANIFEST: &str = ".agent-graph-plug";
const STAGING: &str = ".agent-graph-plug.tmp";

/// The skill folders under `root` that `plug` owns: one per embedded skill,
/// and one per skill in the manifest of an earlier `plug`.
fn owned_dirs(root: &Path) -> Vec<PathBuf> {
    let manifest = fs::read_to_string(root.join(MANIFEST)).unwrap_or_default();
    // Only names with a skill prefix: a changed manifest must not remove other
    // folders. `node-` and `trait-` are the prefixes of the skills from before
    // every skill got the `graph-` prefix. `template-`, `role-`, and `team-`
    // are the prefixes of the skills from before the rename back to graph and
    // node. An upgrade removes all of them.
    let listed = manifest.lines().filter(|name| {
        ["graph-", "node-", "trait-", "template-", "role-", "team-"]
            .iter()
            .any(|prefix| name.starts_with(prefix))
            && !name.contains(['/', '\\'])
    });
    let mut dirs: Vec<PathBuf> = embedded_names().map(|name| root.join(name)).collect();
    dirs.extend(listed.map(|name| root.join(name)));
    dirs.sort();
    dirs.dedup();
    dirs
}

fn embedded_names() -> impl Iterator<Item = &'static str> {
    SKILLS
        .dirs()
        .filter_map(|dir| dir.path().file_name()?.to_str())
}

/// Write the skills into a staging folder first, so that an error leaves the
/// skills of the earlier `plug` as they were.
fn install(root: &Path, skills: &Dir<'_>) -> Result<()> {
    let staging = root.join(STAGING);
    remove_existing(&staging)?;
    if let Err(error) = write_dir(skills, &staging) {
        let _ = remove_existing(&staging);
        return Err(error);
    }
    remove(root)?;
    for name in embedded_names() {
        let (from, to) = (staging.join(name), root.join(name));
        fs::rename(&from, &to).with_context(|| format!("cannot move {}", to.display()))?;
    }
    remove_existing(&staging)?;
    let manifest: String = embedded_names().flat_map(|name| [name, "\n"]).collect();
    fs::write(root.join(MANIFEST), manifest)
        .with_context(|| format!("cannot write {}", root.join(MANIFEST).display()))
}

const PLUGIN_MANIFEST: &str = "plugin.json";
/// The keys that `plug` writes into `plugin.json`.
const PLUGIN_MANIFEST_KEYS: [&str; 2] = ["name", "version"];

fn plugin_manifest_body() -> String {
    let manifest = serde_json::json!({
        "name": "agent-graph",
        "version": env!("CARGO_PKG_VERSION"),
    });
    format!("{manifest:#}\n")
}

/// A `plugin.json` is ours if it is a JSON object with the name
/// `agent-graph` and no key that `plug` does not write. The version can
/// differ, so a later binary still finds the file of an earlier one.
fn manifest_is_ours(plugin: &Path) -> bool {
    // A link is the file of the user, and a write would change its target.
    let path = plugin.join(PLUGIN_MANIFEST);
    if path
        .symlink_metadata()
        .is_ok_and(|metadata| metadata.file_type().is_symlink())
    {
        return false;
    }
    let Ok(body) = fs::read_to_string(&path) else {
        return false;
    };
    let Ok(serde_json::Value::Object(manifest)) = serde_json::from_str(&body) else {
        return false;
    };
    manifest.get("name").and_then(|name| name.as_str()) == Some("agent-graph")
        && manifest
            .keys()
            .all(|key| PLUGIN_MANIFEST_KEYS.contains(&key.as_str()))
}

/// A `plugin.json` that differs from ours is the file of the user: it stays.
/// Returns the path and the reason when it stays.
fn write_manifest(plugin: &Path) -> Result<Option<(PathBuf, &'static str)>> {
    let path = plugin.join(PLUGIN_MANIFEST);
    if let Ok(metadata) = path.symlink_metadata()
        && !manifest_is_ours(plugin)
    {
        let reason = if metadata.file_type().is_symlink() {
            "a symbolic link"
        } else {
            "a file of the user"
        };
        return Ok(Some((path, reason)));
    }
    fs::write(&path, plugin_manifest_body())
        .with_context(|| format!("cannot write {}", path.display()))?;
    Ok(None)
}

/// Removes the manifest if it is ours, then the plugin
/// folder if nothing else is in it, so that a file of the user stays.
fn remove_manifest(plugin: &Path) -> Result<bool> {
    let removed = manifest_is_ours(plugin) && remove_existing(&plugin.join(PLUGIN_MANIFEST))?;
    // `skills/` is empty after `remove`. A failure here means the user put
    // something else in the folder, and it stays.
    let _ = fs::remove_dir(plugin.join("skills"));
    let _ = fs::remove_dir(plugin);
    Ok(removed)
}

fn remove(root: &Path) -> Result<bool> {
    let mut removed = false;
    for dir in owned_dirs(root) {
        removed |= remove_existing(&dir)?;
    }
    remove_existing(&root.join(MANIFEST))?;
    Ok(removed)
}

fn write_dir(dir: &Dir<'_>, dest: &Path) -> Result<()> {
    fs::create_dir_all(dest).with_context(|| format!("cannot create {}", dest.display()))?;
    for file in dir.files() {
        let target = dest.join(file.path().file_name().expect("embedded file has a name"));
        fs::write(&target, file.contents())
            .with_context(|| format!("cannot write {}", target.display()))?;
    }
    for sub in dir.dirs() {
        let name = sub.path().file_name().expect("embedded dir has a name");
        write_dir(sub, &dest.join(name))?;
    }
    Ok(())
}

/// A link is removed as a link, so its target (for example a skill source in
/// a repo) stays as it is.
fn remove_existing(path: &Path) -> Result<bool> {
    let Ok(metadata) = path.symlink_metadata() else {
        return Ok(false);
    };
    if metadata.is_dir() {
        fs::remove_dir_all(path)
    } else {
        fs::remove_file(path)
    }
    .with_context(|| format!("cannot remove {}", path.display()))?;
    Ok(true)
}
