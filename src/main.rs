mod frontmatter;
mod include;
mod init;
mod markdown;
mod role;
mod state;

use std::env;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Result, anyhow};
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
    /// Write the default roles and pods into `<dir>/.agent-roles/`. Does
    /// nothing if `.agent-roles` exists.
    Init {
        /// The folder to write into. The default is the current directory.
        dir: Option<PathBuf>,
    },
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
