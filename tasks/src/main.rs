use std::process::ExitCode;

use clap::{Parser, Subcommand};
use xshell::Shell;

mod ci;
mod clean;
mod coverage;
mod fmt;
mod install;
mod lint;
mod release;
mod run;
mod test;
mod util;

/// Task result; any `Err` is printed and turns into a non-zero exit.
pub(crate) type TaskOutcome = Result<(), Box<dyn std::error::Error>>;

/// Project task runner. Run `cargo task <task>`.
#[derive(Parser)]
#[command(bin_name = "cargo task")]
struct Cli {
    #[command(subcommand)]
    task: Task,
}

/// Variant doc comments *are* the `--help` text, so there is no separate
/// usage block to drift. clap kebab-cases the names.
#[derive(Subcommand)]
enum Task {
    /// Run the tests.
    Test,
    /// Run the CI gate: fmt --check, clippy, tests.
    Ci,
    /// Format source files.
    Fmt,
    /// Run clippy lints.
    Lint,
    /// Remove build artifacts.
    Clean,
    /// Run the binary (`cargo run`). Extra args go to `agent-graph`
    /// (e.g. `cargo task run list`).
    Run {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Install the binary.
    Install,
    /// Build the release binary.
    Release,
    /// Run tests with coverage.
    Coverage,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let sh = match Shell::new() {
        Ok(sh) => sh,
        Err(error) => {
            util::output::error(&error.to_string());
            return ExitCode::FAILURE;
        }
    };

    let outcome = match cli.task {
        Task::Test => test::run(&sh),
        Task::Ci => ci::run(&sh),
        Task::Fmt => fmt::run(&sh),
        Task::Lint => lint::run(&sh),
        Task::Clean => clean::run(&sh),
        Task::Run { args } => run::run(&sh, &args),
        Task::Install => install::run(&sh),
        Task::Release => release::run(&sh),
        Task::Coverage => coverage::run(&sh),
    };

    match outcome {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            util::output::error(&error.to_string());
            ExitCode::FAILURE
        }
    }
}
