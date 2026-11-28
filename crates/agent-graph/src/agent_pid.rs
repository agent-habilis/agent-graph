//! Find the pid of the agent that runs this command, without `$PPID`.
//!
//! The agent is the first ancestor of this process whose name is not a shell.
//! A command of an agent runs in a shell that the agent starts, so the walk
//! goes shell, then agent. A wrapper that is not a shell and does not replace
//! itself with the command, such as `xargs` or `time`, would be taken for the
//! agent: `--pid auto` prints the process that it found, so a wrong one shows.
//! The process table is a parameter so that a test can give it a table of its
//! own. The system table reads the kernel directly and does not run `ps`. A
//! sandbox can still block these reads. Then the walk fails, and the error
//! says to pass `--pid`.

use anyhow::{Result, bail};

/// The names of the shells that stand between an agent and its command.
const SHELLS: [&str; 12] = [
    "sh", "bash", "zsh", "dash", "fish", "ksh", "ash", "tcsh", "csh", "nu", "pwsh", "elvish",
];

/// The most ancestors the walk visits. It stops a loop in a broken table.
const MAX_DEPTH: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Process {
    pub(crate) ppid: u32,
    pub(crate) name: String,
}

pub(crate) trait ProcessTable {
    /// The process with that pid, or `None` when it is gone or unreadable.
    fn process(&self, pid: u32) -> Option<Process>;
}

/// The agent process: its pid, and its name for the line that `--pid auto`
/// prints.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Agent {
    pub(crate) pid: u32,
    pub(crate) name: String,
}

/// The agent that runs the process `from`.
pub(crate) fn agent_pid(table: &impl ProcessTable, from: u32) -> Result<Agent> {
    let Some(start) = table.process(from) else {
        bail!("cannot read the process {from}; pass --pid");
    };
    let mut pid = start.ppid;
    for _ in 0..MAX_DEPTH {
        // Pid 0 is the kernel. Pid 1 can be the agent itself, in a container.
        if pid == 0 {
            break;
        }
        let Some(process) = table.process(pid) else {
            bail!("cannot read the process {pid}; pass --pid");
        };
        if !is_shell(&process.name) {
            return Ok(Agent {
                pid,
                name: process.name,
            });
        }
        pid = process.ppid;
    }
    bail!("no agent process above process {from}; pass --pid")
}

/// The value of `--pid`: a pid, or `auto` for the walk. `auto` is a literal,
/// so a command line that uses it needs no shell construct such as `$PPID`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PidArg {
    Number(u32),
    Auto,
}

impl std::str::FromStr for PidArg {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, String> {
        if text == "auto" {
            return Ok(Self::Auto);
        }
        text.parse()
            .map(Self::Number)
            .map_err(|_| format!("`{text}` is not a pid or `auto`"))
    }
}

impl PidArg {
    pub(crate) fn resolve(self) -> Result<u32> {
        match self {
            Self::Number(pid) => Ok(pid),
            Self::Auto => {
                let agent = find()?;
                eprintln!(
                    "agent-graph: --pid auto is process {} ({})",
                    agent.pid, agent.name
                );
                Ok(agent.pid)
            }
        }
    }
}

/// The agent that runs this process.
fn find() -> Result<Agent> {
    agent_pid(&SystemTable, std::process::id())
}

/// A login shell has a leading `-` in its name, and a name can be a path.
fn is_shell(name: &str) -> bool {
    let base = name.rsplit('/').next().unwrap_or(name);
    SHELLS.contains(&base.trim_start_matches('-'))
}

#[derive(Debug)]
struct SystemTable;

#[cfg(target_os = "macos")]
impl ProcessTable for SystemTable {
    fn process(&self, pid: u32) -> Option<Process> {
        use libproc::bsd_info::BSDInfo;
        use libproc::proc_pid;

        let info = proc_pid::pidinfo::<BSDInfo>(i32::try_from(pid).ok()?, 0).ok()?;
        // `pbi_name` is the long name. It is empty for most processes.
        let name = if info.pbi_name[0] == 0 {
            c_name(&info.pbi_comm)
        } else {
            c_name(&info.pbi_name)
        };
        Some(Process {
            ppid: info.pbi_ppid,
            name,
        })
    }
}

/// The text of a C string in a fixed buffer, up to the first NUL.
#[cfg(target_os = "macos")]
fn c_name(buffer: &[std::ffi::c_char]) -> String {
    let bytes: Vec<u8> = buffer
        .iter()
        .take_while(|&&byte| byte != 0)
        .map(|&byte| byte.cast_unsigned())
        .collect();
    String::from_utf8_lossy(&bytes).into_owned()
}

#[cfg(target_os = "linux")]
impl ProcessTable for SystemTable {
    fn process(&self, pid: u32) -> Option<Process> {
        let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
        parse_proc_stat(&stat)
    }
}

/// `pid (name) state ppid ...`. The name can hold spaces and `)`, so the
/// fields start after the last `)`.
#[cfg(any(target_os = "linux", test))]
fn parse_proc_stat(stat: &str) -> Option<Process> {
    let open = stat.find('(')?;
    let close = stat.rfind(')')?;
    let name = stat.get(open + 1..close)?.to_string();
    let ppid = stat
        .get(close + 1..)?
        .split_whitespace()
        .nth(1)?
        .parse()
        .ok()?;
    Some(Process { ppid, name })
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
impl ProcessTable for SystemTable {
    fn process(&self, _pid: u32) -> Option<Process> {
        None
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    struct Table(HashMap<u32, Process>);

    impl Table {
        /// `(pid, ppid, name)` for each process.
        fn of(rows: &[(u32, u32, &str)]) -> Self {
            Self(
                rows.iter()
                    .map(|&(pid, ppid, name)| {
                        (
                            pid,
                            Process {
                                ppid,
                                name: name.to_string(),
                            },
                        )
                    })
                    .collect(),
            )
        }
    }

    impl ProcessTable for Table {
        fn process(&self, pid: u32) -> Option<Process> {
            self.0.get(&pid).cloned()
        }
    }

    #[test]
    fn pid_arg_parses_a_number_or_auto() {
        assert_eq!("42".parse(), Ok(PidArg::Number(42)));
        assert_eq!("auto".parse(), Ok(PidArg::Auto));
        for text in ["", "Auto", "-1", "4x", "auto "] {
            let error = text.parse::<PidArg>().unwrap_err();
            assert!(error.contains("a pid or `auto`"), "{text:?}: {error}");
        }
    }

    #[test]
    fn a_number_resolves_to_itself() {
        assert_eq!(PidArg::Number(7).resolve().unwrap(), 7);
    }

    #[test]
    fn the_agent_is_the_parent_of_the_shell() {
        let table = Table::of(&[
            (10, 1, "terminal"),
            (20, 10, "claude"),
            (30, 20, "zsh"),
            (40, 30, "agent-graph"),
        ]);
        assert_eq!(agent_pid(&table, 40).unwrap().pid, 20);
    }

    #[test]
    fn the_agent_is_the_parent_when_the_shell_replaces_itself_with_the_command() {
        let table = Table::of(&[
            (10, 1, "terminal"),
            (20, 10, "claude"),
            (40, 20, "agent-graph"),
        ]);
        assert_eq!(agent_pid(&table, 40).unwrap().pid, 20);
    }

    #[test]
    fn the_walk_skips_each_shell_in_a_chain_and_a_login_shell_name() {
        let table = Table::of(&[
            (10, 1, "terminal"),
            (20, 10, "agy"),
            (30, 20, "-zsh"),
            (35, 30, "/bin/bash"),
            (38, 35, "sh"),
            (40, 38, "agent-graph"),
        ]);
        assert_eq!(agent_pid(&table, 40).unwrap().pid, 20);
    }

    #[test]
    fn each_shell_name_is_a_shell() {
        for name in [
            "sh",
            "bash",
            "zsh",
            "dash",
            "fish",
            "ksh",
            "ash",
            "tcsh",
            "csh",
            "nu",
            "pwsh",
            "elvish",
            "-bash",
            "/usr/bin/fish",
        ] {
            assert!(is_shell(name), "{name}");
        }
        for name in ["claude", "node", "shell", "bashful", "zshrc", "agy", ""] {
            assert!(!is_shell(name), "{name}");
        }
    }

    #[test]
    fn the_walk_stops_at_pid_0_and_asks_for_a_pid() {
        let table = Table::of(&[(1, 0, "sh"), (20, 1, "zsh"), (40, 20, "agent-graph")]);
        let error = agent_pid(&table, 40).unwrap_err().to_string();
        assert!(error.contains("no agent process"), "{error}");
        assert!(error.contains("--pid"), "{error}");
    }

    #[test]
    fn the_agent_can_be_pid_1_in_a_container() {
        let table = Table::of(&[(1, 0, "claude"), (20, 1, "zsh"), (40, 20, "agent-graph")]);
        let agent = agent_pid(&table, 40).unwrap();
        assert_eq!((agent.pid, agent.name.as_str()), (1, "claude"));
    }

    #[test]
    fn an_unreadable_process_is_an_error_that_names_it() {
        let table = Table::of(&[(40, 20, "agent-graph")]);
        let parent = agent_pid(&table, 40).unwrap_err().to_string();
        assert!(parent.contains("process 20"), "{parent}");
        let start = agent_pid(&table, 99).unwrap_err().to_string();
        assert!(start.contains("process 99"), "{start}");
    }

    #[test]
    fn a_loop_in_the_table_ends_the_walk() {
        let table = Table::of(&[(2, 3, "zsh"), (3, 2, "bash"), (40, 2, "agent-graph")]);
        let error = agent_pid(&table, 40).unwrap_err().to_string();
        assert!(error.contains("no agent process"), "{error}");
    }

    #[test]
    fn a_proc_stat_name_can_hold_spaces_and_a_closing_parenthesis() {
        let process = parse_proc_stat("123 (my (odd) name) S 45 123 123 0 -1").unwrap();
        assert_eq!(process.name, "my (odd) name");
        assert_eq!(process.ppid, 45);
        assert_eq!(parse_proc_stat("garbage"), None);
    }

    #[cfg(unix)]
    #[test]
    fn the_system_table_reads_this_process_and_its_parent() {
        let me = SystemTable
            .process(std::process::id())
            .expect("this process");
        assert_eq!(me.ppid, std::os::unix::process::parent_id());
        assert_ne!(me.name, "");
        let parent = SystemTable.process(me.ppid).expect("the parent");
        assert_ne!(parent.name, "");
    }
}
