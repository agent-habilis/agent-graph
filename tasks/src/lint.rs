use xshell::{Shell, cmd};

use crate::TaskOutcome;

pub(crate) fn run(sh: &Shell) -> TaskOutcome {
    // `--workspace`: clippy lints only the packages it is given. Without it
    // the tasks crate is compiled as a member but never linted.
    cmd!(sh, "cargo clippy --workspace --all-targets -- -D warnings")
        .quiet()
        .run()?;
    Ok(())
}
