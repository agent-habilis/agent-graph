use xshell::{Shell, cmd};

use crate::TaskOutcome;
use crate::util::output;

pub(crate) fn run(sh: &Shell) -> TaskOutcome {
    output::status("Building", "release binary");
    cmd!(sh, "cargo build -p agent-role --release")
        .quiet()
        .run()?;
    output::status("Built", "target/release/agent-role");
    Ok(())
}
