use xshell::{Shell, cmd};

use crate::TaskOutcome;
use crate::util::{output, repo_root};

pub(crate) fn run(sh: &Shell) -> TaskOutcome {
    output::status("Installing", "agent-role");
    // Absolute, not `--path .`: no task changes directory, so a relative path
    // would only resolve when invoked from the workspace root.
    let pkg = repo_root();
    // `--force`: the crate version rarely changes between builds, and without
    // it `cargo install` treats "already installed" as up-to-date and skips
    // the rebuild, leaving a stale binary in place.
    //
    // `--locked`: without it `cargo install` ignores Cargo.lock and resolves
    // afresh on the host, so a registry release after the lock was cut can
    // change the build.
    cmd!(sh, "cargo install --path {pkg} --force --locked")
        .quiet()
        .run()?;
    output::status("Installed", "~/.cargo/bin/agent-role");
    Ok(())
}
