//! `--pid auto`, on Windows only.
//!
//! The skills pass the agent process to `--pid`, which names the state file
//! of the statusline. On Unix that is the shell's `$PPID`. Under Git Bash on
//! Windows `$PPID` is 1, because the agent is not an MSYS process, and the
//! shell cannot name the Windows pid. So on Windows `--pid` takes `auto`, and
//! the binary finds the agent itself: the first ancestor of this process that
//! is not a shell. Every command of a skill runs under the same agent, so each
//! one finds the same pid, and the same file. agent-gossip does the same in
//! `crates/agent-gossip/src/cli/auto.rs`.
//!
//! The processes come from `sysinfo`, so this crate has no `unsafe` code.

use std::collections::HashMap;

use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System};

/// The image names that `auto` walks past, compared without case and without
/// `.exe`. The shells of Git Bash, MSYS2 and Cygwin, the shells of Windows,
/// and this binary.
const PASSED: [&str; 14] = [
    "bash",
    "sh",
    "dash",
    "zsh",
    "ksh",
    "mksh",
    "fish",
    "tcsh",
    "csh",
    "busybox",
    "cmd",
    "powershell",
    "pwsh",
    "agent-graph",
];

/// How far the walk goes up before it gives up.
const MAX_DEPTH: usize = 16;

/// One process, as the walk sees it.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Entry {
    /// The parent pid. `None` when the process has no parent.
    parent: Option<u32>,
    /// The image name, such as `bash.exe`.
    name: String,
    /// The start time in seconds since the epoch, or 0 when it cannot be
    /// read.
    start: u64,
}

/// The value parser of `--pid` on Windows: a pid, or `auto` for the agent
/// process.
pub(crate) fn pid(value: &str) -> Result<u32, String> {
    if value == "auto" {
        agent_pid(&processes()?, std::process::id())
    } else {
        value.parse().map_err(|error| format!("{error}"))
    }
}

/// Every running process, by pid.
fn processes() -> Result<HashMap<u32, Entry>, String> {
    let mut system =
        System::new().map_err(|error| format!("the processes cannot be listed: {error}"))?;
    system.refresh_processes_specifics(ProcessesToUpdate::All, true, ProcessRefreshKind::nothing());
    Ok(system
        .processes()
        .iter()
        .map(|(pid, process)| {
            let entry = Entry {
                parent: process.parent().map(sysinfo::Pid::as_u32),
                name: process.name().to_string_lossy().into_owned(),
                start: process.start_time(),
            };
            (pid.as_u32(), entry)
        })
        .collect())
}

/// Whether the walk passes the process with image name `name`.
fn passed(name: &str) -> bool {
    let stem = name
        .len()
        .checked_sub(4)
        .filter(|&end| name.is_char_boundary(end) && name[end..].eq_ignore_ascii_case(".exe"))
        .map_or(name, |end| &name[..end]);
    PASSED
        .iter()
        .any(|passed| stem.eq_ignore_ascii_case(passed))
}

/// The agent process above `start`: its first ancestor that is not a shell.
///
/// # Errors
/// A parent is missing from `processes` or exited, its image name cannot be
/// read, the walk reaches pid 0 or 4 (the idle process and the system), a
/// parent started after its child (its pid was reused), or the walk passes
/// [`MAX_DEPTH`] shells. It never falls back to the last pid it reached.
fn agent_pid(processes: &HashMap<u32, Entry>, start: u32) -> Result<u32, String> {
    let mut current = start;
    for _ in 0..MAX_DEPTH {
        let child = processes
            .get(&current)
            .ok_or_else(|| format!("pid {current} is gone, so the agent process is unknown"))?;
        let parent = child.parent.ok_or_else(|| {
            format!("pid {current} has no parent, so the agent process is unknown")
        })?;
        if parent == 0 || parent == 4 {
            return Err(format!(
                "pid {current} has no agent above it (its parent is pid {parent})"
            ));
        }
        let entry = processes.get(&parent).ok_or_else(|| {
            format!("the parent of pid {current} (pid {parent}) is gone, so the agent process is unknown")
        })?;
        if entry.start != 0 && child.start != 0 && entry.start > child.start {
            return Err(format!(
                "the parent of pid {current} (pid {parent}) exited and its pid was reused, so the agent process is unknown"
            ));
        }
        // sysinfo names a process whose image name it cannot read
        // `<no name> Process <pid>`.
        if entry.name.starts_with("<no name>") {
            return Err(format!(
                "the image name of pid {parent} cannot be read, so the agent process is unknown"
            ));
        }
        if !passed(&entry.name) {
            return Ok(parent);
        }
        current = parent;
    }
    Err(format!(
        "more than {MAX_DEPTH} shells above this process, so the agent process is unknown"
    ))
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::{Entry, MAX_DEPTH, agent_pid, passed};

    /// A process table from `(pid, parent, name, start)`.
    fn table(rows: &[(u32, Option<u32>, &str, u64)]) -> HashMap<u32, Entry> {
        rows.iter()
            .map(|&(pid, parent, name, start)| {
                let entry = Entry {
                    parent,
                    name: name.to_owned(),
                    start,
                };
                (pid, entry)
            })
            .collect()
    }

    #[test]
    fn passed_ignores_case_and_exe() {
        assert!(passed("bash.exe"));
        assert!(passed("PowerShell.EXE"));
        assert!(passed("agent-graph.exe"));
        assert!(passed("sh"));
        assert!(!passed("claude.exe"));
        assert!(!passed("node.exe"));
        assert!(!passed("bashful.exe"));
    }

    #[test]
    fn agent_pid_walks_past_the_shells() {
        let processes = table(&[
            (100, Some(80), "agent-graph.exe", 30),
            (80, Some(60), "bash.exe", 20),
            (60, Some(40), "bash.exe", 20),
            (40, Some(20), "claude.exe", 10),
            (20, Some(4), "explorer.exe", 1),
        ]);
        assert_eq!(agent_pid(&processes, 100), Ok(40));
    }

    #[test]
    fn agent_pid_stops_at_the_system() {
        let processes = table(&[
            (100, Some(80), "agent-graph.exe", 30),
            (80, Some(4), "cmd.exe", 20),
        ]);
        let error = agent_pid(&processes, 100).unwrap_err();
        assert!(error.contains("no agent above it"), "{error}");
    }

    #[test]
    fn agent_pid_rejects_a_reused_parent_pid() {
        // pid 80 started after its child: the shell that started 100 exited,
        // and another process got its pid.
        let processes = table(&[
            (100, Some(80), "agent-graph.exe", 30),
            (80, Some(40), "claude.exe", 50),
        ]);
        let error = agent_pid(&processes, 100).unwrap_err();
        assert!(error.contains("reused"), "{error}");
    }

    #[test]
    fn agent_pid_rejects_a_parent_without_a_name() {
        let processes = table(&[
            (100, Some(80), "agent-graph.exe", 30),
            (80, Some(40), "<no name> Process 80", 20),
        ]);
        let error = agent_pid(&processes, 100).unwrap_err();
        assert!(error.contains("image name of pid 80"), "{error}");
    }

    #[test]
    fn agent_pid_rejects_a_missing_parent() {
        let processes = table(&[(100, Some(80), "agent-graph.exe", 30)]);
        let error = agent_pid(&processes, 100).unwrap_err();
        assert!(error.contains("is gone"), "{error}");
    }

    #[test]
    fn agent_pid_gives_up_after_max_depth_shells() {
        let mut rows = vec![(1000, Some(999), "agent-graph.exe", 100)];
        for pid in 0..=u32::try_from(MAX_DEPTH).unwrap() {
            rows.push((999 - pid, Some(998 - pid), "bash.exe", 100));
        }
        let processes = table(&rows);
        let error = agent_pid(&processes, 1000).unwrap_err();
        assert!(error.contains("more than"), "{error}");
    }

    #[test]
    fn pid_auto_finds_a_process_above_this_one() {
        let pid = super::pid("auto").expect("the test has an agent above it");
        assert_ne!(pid, std::process::id());
    }

    #[test]
    fn pid_parses_a_number() {
        assert_eq!(super::pid("4000007"), Ok(4_000_007));
        assert!(super::pid("x").is_err());
    }
}
