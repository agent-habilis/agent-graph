mod frontmatter;
mod include;
mod init;
mod markdown;
mod plug;
mod role;
mod state;
mod team;
mod template;

use std::env;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Result, anyhow, bail};
use clap::{Parser, Subcommand};
use regex::Regex;

/// Find and print agent team templates: OKF bundles in `.agent-graph/`
/// folders from the current directory up to `/`. A template holds its roles
/// in `<template>/roles/`. The nearest template with a name wins.
#[derive(Debug, Parser)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// List, print, and check team templates: folders in `.agent-graph/`.
    Template {
        #[command(subcommand)]
        command: TemplateCommand,
    },
    /// List roles, and load one into an agent.
    Role {
        #[command(subcommand)]
        command: RoleCommand,
    },
    /// Show running teams: team templates that run in a gossip.
    Team {
        #[command(subcommand)]
        command: TeamCommand,
    },
    /// Install the team skills into each agent on this machine. Prints one
    /// line per target: state, agent, path (tab-separated).
    Plug {
        /// The agent to install into (repeatable). The default is each
        /// detected agent.
        #[arg(long = "agent", value_enum)]
        agents: Vec<plug::Agent>,
        /// A folder to install into as a skill root (repeatable). With only
        /// `--path`, no agent is touched.
        #[arg(long = "path")]
        paths: Vec<PathBuf>,
    },
    /// Remove the team skills that `plug` installed. Other skills stay.
    Unplug {
        #[arg(long = "agent", value_enum)]
        agents: Vec<plug::Agent>,
        #[arg(long = "path")]
        paths: Vec<PathBuf>,
    },
}

#[derive(Debug, Subcommand)]
enum TemplateCommand {
    /// Print one line per template: name, description, scope, path
    /// (tab-separated).
    List,
    /// Print the template's index.md body, with its `@file` includes
    /// expanded.
    Get { name: String },
    /// Print one line per role of the template: mermaid id, role, `lead` or
    /// `-`, count, preferred model (tab-separated).
    Roles { name: String },
    /// Print the SHA-256 of the template folder: the id of the template
    /// between peers.
    Hash { name: String },
    /// Check one template, or all templates. Prints `path:line: reason` per
    /// error.
    Lint { name: Option<String> },
    /// Write the default template into `<dir>/.agent-graph/`. Does nothing if
    /// `.agent-graph` exists.
    Init {
        /// The folder to write into. The default is the current directory.
        dir: Option<PathBuf>,
    },
}

#[derive(Debug, Subcommand)]
enum RoleCommand {
    /// Print one line per role: name, description, tags, path
    /// (tab-separated). The name is `<template>/<role>`.
    List {
        /// Show only roles that have a tag that matches this regex.
        #[arg(long)]
        tag: Option<String>,
    },
    /// Print the template's body, then the role's body, with their `@file`
    /// includes expanded.
    Up {
        /// The role, as `<template>/<role>`.
        name: String,
        /// The pid of the agent. Records the role in
        /// `/tmp/agent-graph/<pid>.json` for the statusline.
        #[arg(long)]
        pid: Option<u32>,
    },
    /// Remove the role of an agent from the statusline:
    /// `/tmp/agent-graph/<pid>.json`.
    Down {
        /// The pid of the agent.
        #[arg(long)]
        pid: u32,
    },
}

#[derive(Debug, Subcommand)]
enum TeamCommand {
    /// Draw the running team: one box per member in its role, and the
    /// handoffs between them. Prints one `warning:` line per problem to
    /// stderr.
    Topology {
        /// The team instance name, for example `default@main`. The default
        /// is the team of `--me`.
        instance: Option<String>,
        /// The output of `agent-gossip meta get`.
        #[arg(long)]
        meta: PathBuf,
        /// The output of `agent-gossip peers`.
        #[arg(long)]
        peers: PathBuf,
        /// The gossip nickname of this agent.
        #[arg(long)]
        me: String,
        /// Print the Mermaid source instead of the drawing.
        #[arg(long)]
        mermaid: bool,
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
    let mut stdout = io::stdout().lock();
    match cli.command {
        Command::Template { command } => run_template(command, &cwd, &mut stdout),
        Command::Role { command } => run_role(command, &cwd, &mut stdout),
        Command::Team { command } => run_team(command, &cwd, &mut stdout),
        Command::Plug { agents, paths } => print_lines(&mut stdout, &plug::plug(&agents, &paths)?),
        Command::Unplug { agents, paths } => {
            print_lines(&mut stdout, &plug::unplug(&agents, &paths)?)
        }
    }
}

fn run_team(command: TeamCommand, cwd: &Path, stdout: &mut impl Write) -> Result<()> {
    match command {
        TeamCommand::Topology {
            instance,
            meta,
            peers,
            me,
            mermaid,
        } => {
            let gossip = team::Gossip::read(&meta, &peers)?;
            let templates = role::discover_templates(cwd);
            let (source, warnings) = team::draw(&templates, &gossip, instance.as_deref(), &me)?;
            for warning in warnings {
                eprintln!("warning: {warning}");
            }
            let mut text = if mermaid {
                source
            } else {
                team::render_mermaid(&source)?
            };
            if !text.ends_with('\n') {
                text.push('\n');
            }
            write!(stdout, "{text}")?;
        }
    }
    Ok(())
}

fn run_role(command: RoleCommand, cwd: &Path, stdout: &mut impl Write) -> Result<()> {
    let roles = role::discover_roles(cwd);
    match command {
        RoleCommand::List { tag } => {
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
        RoleCommand::Up { name, pid } => {
            let dir = roles
                .get(&name)
                .ok_or_else(|| anyhow!("role `{name}` not found, use `<template>/<role>`"))?;
            let role = role::load(dir)
                .map_err(|reason| anyhow!("role `{name}` is not valid: {reason}"))?;
            let (template_name, role_name) = name.split_once('/').unwrap_or(("", &name));
            let template_dir = dir.join("../..").canonicalize()?;
            let template = template::load(&template_dir)
                .map_err(|reason| anyhow!("template `{template_name}` is not valid: {reason}"))?;
            let mut text = include::expand(&template_dir, &template_dir.join("index.md"))?;
            text += &include::expand(&template_dir, &dir.join("index.md"))?;
            if let Some(pid) = pid {
                state::write(pid, (template_name, &template), (role_name, &role))?;
            }
            write!(stdout, "{text}")?;
        }
        RoleCommand::Down { pid } => state::remove(pid)?,
    }
    Ok(())
}

fn run_template(command: TemplateCommand, cwd: &Path, stdout: &mut impl Write) -> Result<()> {
    let templates = role::discover_templates(cwd);
    let find = |name: &str| {
        templates
            .get(name)
            .ok_or_else(|| anyhow!("template `{name}` not found"))
    };
    match command {
        TemplateCommand::Init { dir } => {
            let dir = dir.unwrap_or_else(|| cwd.to_path_buf());
            match init::init(&dir)? {
                Some(written) => {
                    for path in written {
                        writeln!(stdout, "{}", path.display())?;
                    }
                }
                None => eprintln!("{} already exists", dir.join(".agent-graph").display()),
            }
        }
        TemplateCommand::List => {
            for (name, dir) in &templates {
                match template::load(dir) {
                    Ok(template) => writeln!(
                        stdout,
                        "{name}\t{}\t{}\t{}",
                        template.description,
                        template.scope,
                        dir.display()
                    )?,
                    Err(reason) => eprintln!("warning: {}: {reason}", dir.display()),
                }
            }
        }
        TemplateCommand::Get { name } => {
            let dir = find(&name)?;
            template::load(dir)
                .map_err(|reason| anyhow!("template `{name}` is not valid: {reason}"))?;
            write!(stdout, "{}", include::expand(dir, &dir.join("index.md"))?)?;
        }
        TemplateCommand::Hash { name } => {
            writeln!(stdout, "{}", template::hash(find(&name)?)?)?;
        }
        TemplateCommand::Roles { name } => {
            let dir = find(&name)?;
            let template = template::load(dir)
                .map_err(|reason| anyhow!("template `{name}` is not valid: {reason}"))?;
            for vertex in template
                .vertices
                .iter()
                .filter(|vertex| vertex.subgraph.is_some())
            {
                let role = role::load(&template::role_dir(dir, vertex)).ok();
                let count = role.as_ref().map_or(1, |role| role.count);
                let model = role.and_then(|role| role.model).unwrap_or_default();
                let lead = if vertex.lead { "lead" } else { "-" };
                writeln!(
                    stdout,
                    "{}\t{name}/{}\t{lead}\t{count}\t{model}",
                    vertex.id, vertex.label
                )?;
            }
        }
        TemplateCommand::Lint { name: only } => {
            let targets: Vec<(&String, &PathBuf)> = match &only {
                Some(name) => vec![(name, find(name)?)],
                None => templates.iter().collect(),
            };
            let mut count = 0;
            for (_, dir) in targets {
                let path = dir.join("index.md");
                let errors = match template::load(dir) {
                    Ok(template) => template::lint(dir, &template),
                    Err(reason) => vec![(1, reason)],
                };
                for (line, reason) in &errors {
                    writeln!(stdout, "{}:{line}: {reason}", path.display())?;
                }
                count += errors.len();
            }
            if count > 0 {
                bail!("{count} errors in templates");
            }
        }
    }
    Ok(())
}

fn print_lines(stdout: &mut impl Write, lines: &[String]) -> Result<()> {
    for line in lines {
        writeln!(stdout, "{line}")?;
    }
    Ok(())
}
