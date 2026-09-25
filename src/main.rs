mod frontmatter;
mod include;
mod init;
mod markdown;
mod role;

use std::env;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Result, anyhow};
use clap::{Parser, Subcommand};
use regex::Regex;

/// Find and print agent roles: OKF bundles in `.agent-roles/roles/` folders
/// from the current directory up to `/`. The nearest role with a name wins.
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
    Get { name: String },
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
    let roles = role::discover(&cwd, "roles");
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
        Command::Get { name } => {
            let dir = roles
                .get(&name)
                .ok_or_else(|| anyhow!("role `{name}` not found"))?;
            role::load(dir).map_err(|reason| anyhow!("role `{name}` is not valid: {reason}"))?;
            write!(stdout, "{}", include::expand(dir, &dir.join("index.md"))?)?;
        }
    }
    Ok(())
}
