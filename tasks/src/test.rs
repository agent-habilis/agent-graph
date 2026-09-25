use xshell::{Shell, cmd};

use crate::TaskOutcome;

pub(crate) fn run(sh: &Shell) -> TaskOutcome {
    // `--no-fail-fast` because cargo otherwise stops at the first failing
    // binary and leaves every later one unreported.
    cmd!(sh, "cargo test --workspace --no-fail-fast")
        .quiet()
        .run()?;
    Ok(())
}
