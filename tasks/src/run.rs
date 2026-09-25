use xshell::{Shell, cmd};

use crate::TaskOutcome;

pub(crate) fn run(sh: &Shell, args: &[String]) -> TaskOutcome {
    cmd!(sh, "cargo run -p agent-graph -- {args...}").run()?;
    Ok(())
}
