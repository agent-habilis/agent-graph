mod init;
mod plug;
mod state;

use std::collections::BTreeMap;
use std::env;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use agent_graph_core::{graph, include, node, traits};
use agent_graph_topology as topology;
use anyhow::{Result, anyhow, bail};
use clap::{Parser, Subcommand};
use regex::Regex;

/// Find and print agent graphs: OKF bundles in `.agent-graph/`
/// folders from the current directory up to `/`. A graph holds its nodes
/// in `<graph>/nodes/`. The nearest graph with a name wins.
#[derive(Debug, Parser)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// List, print, and check graphs: folders in `.agent-graph/`. Draw
    /// running graphs: graphs that run in a gossip.
    Graph {
        #[command(subcommand)]
        command: GraphCommand,
    },
    /// List nodes, and load one into an agent.
    Node {
        #[command(subcommand)]
        command: NodeCommand,
    },
    /// List, check, and print traits: routines in `.agent-graph/traits/`
    /// that an agent runs in a fork.
    Trait {
        #[command(subcommand)]
        command: TraitCommand,
    },
    /// Install the graph skills into each agent on this machine. Prints one
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
    /// Remove the graph skills that `plug` installed. Other skills stay.
    Unplug {
        #[arg(long = "agent", value_enum)]
        agents: Vec<plug::Agent>,
        #[arg(long = "path")]
        paths: Vec<PathBuf>,
    },
}

#[derive(Debug, Subcommand)]
enum GraphCommand {
    /// Print one line per graph: name, description, scope, path
    /// (tab-separated).
    List,
    /// Print the graph's index.md body, with its `@file` includes
    /// expanded.
    Get { name: String },
    /// Print one line per node of the graph: mermaid id, node, `lead` or
    /// `-`, count, preferred model (tab-separated).
    Nodes { name: String },
    /// Print the SHA-256 of the graph folder: the id of the graph
    /// between peers.
    Hash { name: String },
    /// Check one graph, or all graphs. Prints `path:line: reason` per
    /// error.
    Lint { name: Option<String> },
    /// Write the default graphs into `<dir>/.agent-graph/`. Does nothing if
    /// `.agent-graph` exists.
    Init {
        /// The folder to write into. The default is the current directory.
        dir: Option<PathBuf>,
    },
    /// Draw the running graph: one box per peer in its node, and the
    /// edges between them. Prints one `warning:` line per problem to
    /// stderr.
    Topology {
        /// The graph instance name, for example `squad@main`. The default
        /// is the graph of `--me`.
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

#[derive(Debug, Subcommand)]
enum TraitCommand {
    /// Print one line per trait: name, description, writes, path
    /// (tab-separated).
    List,
    /// Check a trait file, or the `index.md` of a trait folder. Print its
    /// description, writes, and path (tab-separated).
    Check { path: PathBuf },
    /// Print the trait's body, with its `@file` includes expanded. A read-only
    /// trait gets a `## Boundaries` section.
    Show {
        /// The trait name, or a path to a trait file (it holds a `/` or ends
        /// in `.md`).
        name: String,
        /// The node of the agent, as `<graph>/<node>`. A trait by name must
        /// be in the node's `traits` list.
        #[arg(long)]
        node: Option<String>,
    },
}

const READ_ONLY: &str = "## Boundaries\n\n- No changes to files that git tracks, and no commits.\n";

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
        Command::Trait { command } => run_trait(command, &cwd, &mut stdout),
        Command::Plug { agents, paths } => print_lines(&mut stdout, &plug::plug(&agents, &paths)?),
        Command::Unplug { agents, paths } => {
            print_lines(&mut stdout, &plug::unplug(&agents, &paths)?)
        }
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

fn run_trait(command: TraitCommand, cwd: &Path, stdout: &mut impl Write) -> Result<()> {
    let found = traits::discover(cwd);
    match command {
        TraitCommand::List => {
            for (name, dir) in &found {
                match traits::load(&dir.join("index.md")) {
                    Ok(item) => writeln!(
                        stdout,
                        "{name}\t{}\t{}\t{}",
                        item.description,
                        item.writes,
                        dir.display()
                    )?,
                    Err(reason) => eprintln!("warning: {}: {reason}", dir.display()),
                }
            }
        }
        TraitCommand::Check { path } => {
            let file = if path.is_dir() {
                path.join("index.md")
            } else {
                path
            };
            let item = traits::load(&file)
                .map_err(|reason| anyhow!("{} is not a valid trait: {reason}", file.display()))?;
            writeln!(
                stdout,
                "{}\t{}\t{}",
                item.description,
                item.writes,
                file.display()
            )?;
        }
        TraitCommand::Show { name, node } => {
            let is_md = Path::new(&name)
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("md"));
            let file = if name.contains('/') || is_md {
                PathBuf::from(&name)
            } else {
                if let Some(node_name) = node {
                    let dir = node::discover_nodes(cwd)
                        .remove(&node_name)
                        .ok_or_else(|| anyhow!("node `{node_name}` not found"))?;
                    let listed = node::load(&dir)
                        .map_err(|reason| anyhow!("node `{node_name}` is not valid: {reason}"))?;
                    if !listed.traits.contains(&name) {
                        bail!("node `{node_name}` does not have trait `{name}`");
                    }
                }
                found
                    .get(&name)
                    .ok_or_else(|| anyhow!("trait `{name}` not found"))?
                    .join("index.md")
            };
            let item = traits::load(&file)
                .map_err(|reason| anyhow!("trait `{name}` is not valid: {reason}"))?;
            let root = file.parent().unwrap_or(Path::new("."));
            write!(stdout, "{}", include::expand(root, &file)?)?;
            if !item.writes {
                write!(stdout, "\n{READ_ONLY}")?;
            }
        }
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
                let lead = if vertex.lead { "lead" } else { "-" };
                writeln!(
                    stdout,
                    "{}\t{name}/{}\t{lead}\t{count}\t{model}",
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
        GraphCommand::Topology {
            instance,
            meta,
            peers,
            me,
            mermaid,
        } => {
            let gossip = topology::Gossip::read(&meta, &peers)?;
            draw_topology(&graphs, &gossip, instance.as_deref(), &me, mermaid, stdout)?;
        }
    }
    Ok(())
}

/// Draw the running graph `instance`, or the graph of `me`, and print its
/// warnings to stderr.
fn draw_topology(
    graphs: &BTreeMap<String, PathBuf>,
    gossip: &topology::Gossip,
    instance: Option<&str>,
    me: &str,
    mermaid: bool,
    stdout: &mut impl Write,
) -> Result<()> {
    let (source, warnings) = topology::draw(graphs, gossip, instance, me)?;
    for warning in warnings {
        eprintln!("warning: {warning}");
    }
    let mut text = if mermaid {
        source
    } else {
        topology::render_mermaid(&source)?
    };
    if !text.ends_with('\n') {
        text.push('\n');
    }
    write!(stdout, "{text}")?;
    Ok(())
}

fn print_lines(stdout: &mut impl Write, lines: &[String]) -> Result<()> {
    for line in lines {
        writeln!(stdout, "{line}")?;
    }
    Ok(())
}
