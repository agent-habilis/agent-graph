//! `agent-graph plug` / `unplug`: install or remove the graph skills in each
//! agent. The skills are rendered by `build.rs` and embedded, so the binary
//! needs no repo checkout. Ported from agent-gossip `src/cli/plug.rs`.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use include_dir::{Dir, include_dir};

static SKILLS: Dir<'_> = include_dir!("$OUT_DIR/skills");

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
}

impl Agent {
    const ALL: [Agent; 5] = [
        Agent::ClaudeCode,
        Agent::Pi,
        Agent::Codex,
        Agent::Cursor,
        Agent::OpenCode,
    ];

    fn label(self) -> &'static str {
        match self {
            Agent::ClaudeCode => "claude-code",
            Agent::Pi => "pi",
            Agent::Codex => "codex",
            Agent::Cursor => "cursor",
            Agent::OpenCode => "opencode",
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
        })
    }

    fn skills_dir(self, home: &Path) -> PathBuf {
        match self {
            Agent::Pi => home.join(".pi/agent/skills"),
            Agent::ClaudeCode | Agent::Codex | Agent::Cursor | Agent::OpenCode => {
                self.agent_dir(home).join("skills")
            }
        }
    }

    fn detected(self, home: &Path) -> bool {
        self.agent_dir(home).exists()
    }

    fn installed(self, home: &Path) -> bool {
        owned_dirs(&self.skills_dir(home))
            .iter()
            .any(|dir| dir.symlink_metadata().is_ok())
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
            install(&dir)?;
            lines.push(format!("installed\t{}\t{}", agent.label(), dir.display()));
        } else {
            lines.push(format!("skipped\t{}\tnot detected", agent.label()));
        }
    }
    for path in paths {
        install(path)?;
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
        let state = if remove(&dir)? { "removed" } else { "skipped" };
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
    // Only `graph-*` names: a changed manifest must not remove other folders.
    let listed = manifest
        .lines()
        .filter(|name| name.starts_with("graph-") && !name.contains(['/', '\\']));
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
fn install(root: &Path) -> Result<()> {
    let staging = root.join(STAGING);
    remove_existing(&staging)?;
    if let Err(error) = write_dir(&SKILLS, &staging) {
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
