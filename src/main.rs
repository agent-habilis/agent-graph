mod frontmatter;
mod include;
mod init;
mod markdown;
mod pod;
mod role;
mod state;

use std::collections::BTreeMap;
use std::env;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Result, anyhow, bail};
use clap::{Parser, Subcommand};
use regex::Regex;

/// Find and print agent roles: OKF bundles in `.agent-roles/roles/` and
/// `.agent-roles/pods/<pod>/roles/` folders from the current directory up to
/// `/`. The nearest role with a name wins.
#[derive(Debug, Parser)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Print one line per role: name, description, tags, path (tab-separated).
    List {
        /// Show only roles that have a tag that matches this regex.
        #[arg(long)]
        tag: Option<String>,
    },
    /// Print the role's index.md body, with its `@file` includes expanded.
    /// For a pod role (`<pod>/<role>`), print the pod's body first.
    Get {
        name: String,
        /// The pid of the agent. Records the role in
        /// `/tmp/agent-roles/<pid>.json` for the statusline.
        #[arg(long)]
        pid: Option<u32>,
    },
    /// List, print, and check pods: folders in `.agent-roles/pods/`.
    Pod {
        #[command(subcommand)]
        command: PodCommand,
    },
    /// Write the default roles and pods into `<dir>/.agent-roles/`. Does
    /// nothing if `.agent-roles` exists.
    Init {
        /// The folder to write into. The default is the current directory.
        dir: Option<PathBuf>,
    },
}

#[derive(Debug, Subcommand)]
enum PodCommand {
    /// Print one line per pod: name, description, scope, path (tab-separated).
    List,
    /// Print the pod's index.md body, with its `@file` includes expanded.
    Get { name: String },
    /// Print one line per pod node: node, role, class (tab-separated).
    Nodes { name: String },
    /// Check one pod, or all pods. Prints `path:line: reason` per error.
    Lint { name: Option<String> },
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("error: {err:#}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> Result<()> {
    let cwd = env::current_dir()?;
    let roles = role::discover_roles(&cwd);
    let mut stdout = io::stdout().lock();
    match cli.command {
        Command::Pod { command } => run_pod(command, &cwd, &roles, &mut stdout)?,
        Command::Init { dir } => {
            let dir = dir.unwrap_or(cwd);
            match init::init(&dir)? {
                Some(written) => {
                    for path in written {
                        writeln!(stdout, "{}", path.display())?;
                    }
                }
                None => eprintln!("{} already exists", dir.join(".agent-roles").display()),
            }
        }
        Command::List { tag } => {
            let tag = tag.map(|pattern| Regex::new(&pattern)).transpose()?;
            for (name, dir) in &roles {
                let role = match role::load(dir) {
                    Ok(role) => role,
                    Err(reason) => {
                        eprintln!("warning: {}: {reason}", dir.display());
                        continue;
                    }
                };
                if let Some(tag) = &tag
                    && !role.tags.iter().any(|item| tag.is_match(item))
                {
                    continue;
                }
                writeln!(
                    stdout,
                    "{name}\t{}\t{}\t{}",
                    role.description,
                    role.tags.join(","),
                    dir.display()
                )?;
            }
        }
        Command::Get { name, pid } => {
            let dir = roles
                .get(&name)
                .ok_or_else(|| anyhow!("role `{name}` not found"))?;
            let role = role::load(dir)
                .map_err(|reason| anyhow!("role `{name}` is not valid: {reason}"))?;
            let (pod, role_name) = match name.split_once('/') {
                Some((pod_name, role_name)) => {
                    let pod_dir = dir.join("../..").canonicalize()?;
                    let pod = role::load_pod(&pod_dir)
                        .map_err(|reason| anyhow!("pod `{pod_name}` is not valid: {reason}"))?;
                    (Some((pod_name, pod, pod_dir)), role_name)
                }
                None => (None, name.as_str()),
            };
            let root = pod.as_ref().map_or(dir, |(_, _, pod_dir)| pod_dir);
            let mut text = String::new();
            if pod.is_some() {
                text += &include::expand(root, &root.join("index.md"))?;
            }
            text += &include::expand(root, &dir.join("index.md"))?;
            if let Some(pid) = pid {
                let pod = pod.as_ref().map(|(pod_name, pod, _)| (*pod_name, pod));
                state::write(pid, (role_name, &role), pod)?;
            }
            write!(stdout, "{text}")?;
        }
    }
    Ok(())
}

fn run_pod(
    command: PodCommand,
    cwd: &Path,
    roles: &BTreeMap<String, PathBuf>,
    stdout: &mut impl Write,
) -> Result<()> {
    let pods = role::discover(cwd, "pods");
    let find = |name: &str| {
        pods.get(name)
            .ok_or_else(|| anyhow!("pod `{name}` not found"))
    };
    match command {
        PodCommand::List => {
            for (name, dir) in &pods {
                match pod::load(dir) {
                    Ok(pod) => writeln!(
                        stdout,
                        "{name}\t{}\t{}\t{}",
                        pod.description,
                        pod.scope,
                        dir.display()
                    )?,
                    Err(reason) => eprintln!("warning: {}: {reason}", dir.display()),
                }
            }
        }
        PodCommand::Get { name } => {
            let dir = find(&name)?;
            pod::load(dir).map_err(|reason| anyhow!("pod `{name}` is not valid: {reason}"))?;
            write!(stdout, "{}", include::expand(dir, &dir.join("index.md"))?)?;
        }
        PodCommand::Nodes { name } => {
            let dir = find(&name)?;
            let pod =
                pod::load(dir).map_err(|reason| anyhow!("pod `{name}` is not valid: {reason}"))?;
            for node in pod.nodes.iter().filter(|node| node.subgraph.is_some()) {
                let role =
                    pod::role_of(&name, dir, node, roles).unwrap_or_else(|| node.label.clone());
                let class = node.class.map_or("none", pod::Class::name);
                writeln!(stdout, "{}\t{role}\t{class}", node.id)?;
            }
        }
        PodCommand::Lint { name: only } => {
            let targets: Vec<(&String, &PathBuf)> = match &only {
                Some(name) => vec![(name, find(name)?)],
                None => pods.iter().collect(),
            };
            let mut count = 0;
            for (name, dir) in targets {
                let path = dir.join("index.md");
                let errors = match pod::load(dir) {
                    Ok(pod) => pod::lint(name, dir, &pod, roles),
                    Err(reason) => vec![(1, reason)],
                };
                for (line, reason) in &errors {
                    writeln!(stdout, "{}:{line}: {reason}", path.display())?;
                }
                count += errors.len();
            }
            if count > 0 {
                bail!("{count} errors in pods");
            }
        }
    }
    Ok(())
}
