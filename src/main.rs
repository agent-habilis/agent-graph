mod frontmatter;
mod graph;
mod include;
mod init;
mod markdown;
mod node;
mod state;

use std::env;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Result, anyhow, bail};
use clap::{Parser, Subcommand};
use regex::Regex;

/// Find and print agent graphs: OKF bundles in `.agent-graph/` folders from
/// the current directory up to `/`. A graph holds its nodes in
/// `<graph>/nodes/`. The nearest graph with a name wins.
#[derive(Debug, Parser)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// List, print, and check graphs: folders in `.agent-graph/`.
    Graph {
        #[command(subcommand)]
        command: GraphCommand,
    },
    /// List nodes, and load one into an agent.
    Node {
        #[command(subcommand)]
        command: NodeCommand,
    },
}

#[derive(Debug, Subcommand)]
enum GraphCommand {
    /// Print one line per graph: name, description, scope, path
    /// (tab-separated).
    List,
    /// Print the graph's index.md body, with its `@file` includes expanded.
    Get { name: String },
    /// Print one line per graph node: mermaid id, node, class, count,
    /// preferred model (tab-separated).
    Nodes { name: String },
    /// Print the SHA-256 of the graph folder: the id of the graph between
    /// peers.
    Hash { name: String },
    /// Check one graph, or all graphs. Prints `path:line: reason` per error.
    Lint { name: Option<String> },
    /// Write the default graph into `<dir>/.agent-graph/`. Does nothing if
    /// `.agent-graph` exists.
    Init {
        /// The folder to write into. The default is the current directory.
        dir: Option<PathBuf>,
    },
}

#[derive(Debug, Subcommand)]
enum NodeCommand {
    /// Print one line per node: name, description, tags, path
    /// (tab-separated). The name is `<graph>/<node>`.
    List {
        /// Show only nodes that have a tag that matches this regex.
        #[arg(long)]
        tag: Option<String>,
    },
    /// Print the graph's body, then the node's body, with their `@file`
    /// includes expanded.
    Up {
        /// The node, as `<graph>/<node>`.
        name: String,
        /// The pid of the agent. Records the node in
        /// `/tmp/agent-graph/<pid>.json` for the statusline.
        #[arg(long)]
        pid: Option<u32>,
    },
    /// Remove the node of an agent from the statusline:
    /// `/tmp/agent-graph/<pid>.json`.
    Down {
        /// The pid of the agent.
        #[arg(long)]
        pid: u32,
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
        Command::Graph { command } => run_graph(command, &cwd, &mut stdout),
        Command::Node { command } => run_node(command, &cwd, &mut stdout),
    }
}

fn run_node(command: NodeCommand, cwd: &Path, stdout: &mut impl Write) -> Result<()> {
    let nodes = node::discover_nodes(cwd);
    match command {
        NodeCommand::List { tag } => {
            let tag = tag.map(|pattern| Regex::new(&pattern)).transpose()?;
            for (name, dir) in &nodes {
                let node = match node::load(dir) {
                    Ok(node) => node,
                    Err(reason) => {
                        eprintln!("warning: {}: {reason}", dir.display());
                        continue;
                    }
                };
                if let Some(tag) = &tag
                    && !node.tags.iter().any(|item| tag.is_match(item))
                {
                    continue;
                }
                writeln!(
                    stdout,
                    "{name}\t{}\t{}\t{}",
                    node.description,
                    node.tags.join(","),
                    dir.display()
                )?;
            }
        }
        NodeCommand::Up { name, pid } => {
            let dir = nodes
                .get(&name)
                .ok_or_else(|| anyhow!("node `{name}` not found, use `<graph>/<node>`"))?;
            let node = node::load(dir)
                .map_err(|reason| anyhow!("node `{name}` is not valid: {reason}"))?;
            let (graph_name, node_name) = name.split_once('/').unwrap_or(("", &name));
            let graph_dir = dir.join("../..").canonicalize()?;
            let graph = graph::load(&graph_dir)
                .map_err(|reason| anyhow!("graph `{graph_name}` is not valid: {reason}"))?;
            let mut text = include::expand(&graph_dir, &graph_dir.join("index.md"))?;
            text += &include::expand(&graph_dir, &dir.join("index.md"))?;
            if let Some(pid) = pid {
                state::write(pid, (graph_name, &graph), (node_name, &node))?;
            }
            write!(stdout, "{text}")?;
        }
        NodeCommand::Down { pid } => state::remove(pid)?,
    }
    Ok(())
}

fn run_graph(command: GraphCommand, cwd: &Path, stdout: &mut impl Write) -> Result<()> {
    let graphs = node::discover_graphs(cwd);
    let find = |name: &str| {
        graphs
            .get(name)
            .ok_or_else(|| anyhow!("graph `{name}` not found"))
    };
    match command {
        GraphCommand::Init { dir } => {
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
        GraphCommand::List => {
            for (name, dir) in &graphs {
                match graph::load(dir) {
                    Ok(graph) => writeln!(
                        stdout,
                        "{name}\t{}\t{}\t{}",
                        graph.description,
                        graph.scope,
                        dir.display()
                    )?,
                    Err(reason) => eprintln!("warning: {}: {reason}", dir.display()),
                }
            }
        }
        GraphCommand::Get { name } => {
            let dir = find(&name)?;
            graph::load(dir).map_err(|reason| anyhow!("graph `{name}` is not valid: {reason}"))?;
            write!(stdout, "{}", include::expand(dir, &dir.join("index.md"))?)?;
        }
        GraphCommand::Hash { name } => {
            writeln!(stdout, "{}", graph::hash(find(&name)?)?)?;
        }
        GraphCommand::Nodes { name } => {
            let dir = find(&name)?;
            let graph = graph::load(dir)
                .map_err(|reason| anyhow!("graph `{name}` is not valid: {reason}"))?;
            for vertex in graph
                .vertices
                .iter()
                .filter(|vertex| vertex.subgraph.is_some())
            {
                let node = node::load(&graph::node_dir(dir, vertex)).ok();
                let count = node.as_ref().map_or(1, |node| node.count);
                let model = node.and_then(|node| node.model).unwrap_or_default();
                let class = vertex.class.map_or("none", graph::Class::name);
                writeln!(
                    stdout,
                    "{}\t{name}/{}\t{class}\t{count}\t{model}",
                    vertex.id, vertex.label
                )?;
            }
        }
        GraphCommand::Lint { name: only } => {
            let targets: Vec<(&String, &PathBuf)> = match &only {
                Some(name) => vec![(name, find(name)?)],
                None => graphs.iter().collect(),
            };
            let mut count = 0;
            for (_, dir) in targets {
                let path = dir.join("index.md");
                let errors = match graph::load(dir) {
                    Ok(graph) => graph::lint(dir, &graph),
                    Err(reason) => vec![(1, reason)],
                };
                for (line, reason) in &errors {
                    writeln!(stdout, "{}:{line}: {reason}", path.display())?;
                }
                count += errors.len();
            }
            if count > 0 {
                bail!("{count} errors in graphs");
            }
        }
    }
    Ok(())
}
