use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System};

use crate::error::WrathError;

/// Result of a successful process kill operation.
#[derive(Debug, PartialEq)]
pub struct KillResult {
    /// Name of the primary process that was killed.
    pub process_name: String,
    /// Total number of processes killed (parent + children).
    pub total_killed: usize,
    /// The port that was freed, if applicable.
    pub port: Option<u16>,
}

impl KillResult {
    /// Format the result message for display.
    pub fn message(&self) -> String {
        let process_word = if self.total_killed == 1 {
            "process"
        } else {
            "processes"
        };

        match self.port {
            Some(port) => format!(
                "[\u{1f525}] Wrath unleashed: Process '{}' ({} {}) using port {} has been obliterated.",
                self.process_name, self.total_killed, process_word, port
            ),
            None => format!(
                "[\u{1f525}] Wrath unleashed: Process '{}' ({} {}) has been obliterated.",
                self.process_name, self.total_killed, process_word
            ),
        }
    }
}

/// Create a refreshed `System` instance with process information loaded.
pub fn create_system() -> System {
    let mut system = System::new();
    system.refresh_processes_specifics(
        ProcessesToUpdate::All,
        true,
        ProcessRefreshKind::everything(),
    );
    system
}

/// Find and kill all processes with the given name, including their children.
pub fn kill_by_name(system: &System, name: &str) -> Result<KillResult, WrathError> {
    let matching_pids: Vec<Pid> = system
        .processes()
        .iter()
        .filter(|(_, process)| {
            let proc_name = process.name().to_string_lossy();
            proc_name == name || proc_name == format!("{name}.exe")
        })
        .map(|(pid, _)| *pid)
        .collect();

    if matching_pids.is_empty() {
        return Err(WrathError::NoProcessWithName(name.to_string()));
    }

    let process_name =
        get_process_name(system, matching_pids[0]).unwrap_or_else(|| name.to_string());

    let mut all_pids_to_kill: Vec<Pid> = Vec::new();
    for pid in &matching_pids {
        all_pids_to_kill.push(*pid);
        collect_children(system, *pid, &mut all_pids_to_kill);
    }

    // Deduplicate
    all_pids_to_kill.sort();
    all_pids_to_kill.dedup();

    let total_killed = kill_processes(system, &all_pids_to_kill)?;

    Ok(KillResult {
        process_name,
        total_killed,
        port: None,
    })
}

/// Find and kill the process using the given port, including its children.
pub fn kill_by_pid(system: &System, pid: u32, port: u16) -> Result<KillResult, WrathError> {
    let sysinfo_pid = Pid::from_u32(pid);

    let process_name =
        get_process_name(system, sysinfo_pid).ok_or(WrathError::NoProcessOnPort(port))?;

    let mut all_pids: Vec<Pid> = vec![sysinfo_pid];
    collect_children(system, sysinfo_pid, &mut all_pids);

    let total_killed = kill_processes(system, &all_pids)?;

    Ok(KillResult {
        process_name,
        total_killed,
        port: Some(port),
    })
}

/// Get the name of a process by its PID.
fn get_process_name(system: &System, pid: Pid) -> Option<String> {
    system
        .process(pid)
        .map(|p| p.name().to_string_lossy().to_string())
}

/// Recursively collect all child process PIDs.
fn collect_children(system: &System, parent_pid: Pid, pids: &mut Vec<Pid>) {
    for (pid, process) in system.processes() {
        if let Some(ppid) = process.parent() {
            if ppid == parent_pid && !pids.contains(pid) {
                pids.push(*pid);
                collect_children(system, *pid, pids);
            }
        }
    }
}

/// Kill all processes in the list. Returns the count of successfully killed processes.
fn kill_processes(system: &System, pids: &[Pid]) -> Result<usize, WrathError> {
    let mut killed = 0;

    for pid in pids {
        if let Some(process) = system.process(*pid) {
            process.kill();
            killed += 1;
        }
    }

    if killed == 0 {
        return Err(WrathError::KillFailed(
            "Could not kill any processes".to_string(),
        ));
    }

    Ok(killed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kill_result_message_single_process_no_port() {
        let result = KillResult {
            process_name: "node".to_string(),
            total_killed: 1,
            port: None,
        };
        assert_eq!(
            result.message(),
            "[\u{1f525}] Wrath unleashed: Process 'node' (1 process) has been obliterated."
        );
    }

    #[test]
    fn test_kill_result_message_multiple_processes_no_port() {
        let result = KillResult {
            process_name: "node".to_string(),
            total_killed: 3,
            port: None,
        };
        assert_eq!(
            result.message(),
            "[\u{1f525}] Wrath unleashed: Process 'node' (3 processes) has been obliterated."
        );
    }

    #[test]
    fn test_kill_result_message_single_process_with_port() {
        let result = KillResult {
            process_name: "node".to_string(),
            total_killed: 1,
            port: Some(3000),
        };
        assert_eq!(
            result.message(),
            "[\u{1f525}] Wrath unleashed: Process 'node' (1 process) using port 3000 has been obliterated."
        );
    }

    #[test]
    fn test_kill_result_message_multiple_processes_with_port() {
        let result = KillResult {
            process_name: "node".to_string(),
            total_killed: 5,
            port: Some(8080),
        };
        assert_eq!(
            result.message(),
            "[\u{1f525}] Wrath unleashed: Process 'node' (5 processes) using port 8080 has been obliterated."
        );
    }

    #[test]
    fn test_kill_result_debug() {
        let result = KillResult {
            process_name: "test".to_string(),
            total_killed: 1,
            port: None,
        };
        let debug_str = format!("{result:?}");
        assert!(debug_str.contains("test"));
    }

    #[test]
    fn test_kill_result_equality() {
        let a = KillResult {
            process_name: "node".to_string(),
            total_killed: 1,
            port: None,
        };
        let b = KillResult {
            process_name: "node".to_string(),
            total_killed: 1,
            port: None,
        };
        assert_eq!(a, b);
    }

    #[test]
    fn test_kill_result_inequality() {
        let a = KillResult {
            process_name: "node".to_string(),
            total_killed: 1,
            port: None,
        };
        let b = KillResult {
            process_name: "node".to_string(),
            total_killed: 2,
            port: None,
        };
        assert_ne!(a, b);
    }

    #[test]
    fn test_create_system_returns_system() {
        let system = create_system();
        // The system should have been created and refreshed; it should have at least
        // the current process.
        assert!(!system.processes().is_empty());
    }

    #[test]
    fn test_kill_by_name_nonexistent_process() {
        let system = create_system();
        let result = kill_by_name(&system, "this_process_definitely_does_not_exist_xyzzy_42");
        assert_eq!(
            result,
            Err(WrathError::NoProcessWithName(
                "this_process_definitely_does_not_exist_xyzzy_42".to_string()
            ))
        );
    }

    #[test]
    fn test_kill_by_pid_nonexistent_process() {
        let system = create_system();
        // PID 999999999 should not exist
        let result = kill_by_pid(&system, 999_999_999, 3000);
        assert_eq!(result, Err(WrathError::NoProcessOnPort(3000)));
    }

    #[test]
    fn test_collect_children_no_children() {
        let system = create_system();
        // Use a PID that doesn't exist - should collect nothing
        let fake_pid = Pid::from_u32(999_999_999);
        let mut pids = Vec::new();
        collect_children(&system, fake_pid, &mut pids);
        assert!(pids.is_empty());
    }

    #[test]
    fn test_get_process_name_nonexistent() {
        let system = create_system();
        let name = get_process_name(&system, Pid::from_u32(999_999_999));
        assert!(name.is_none());
    }

    #[test]
    fn test_get_process_name_current_process() {
        let system = create_system();
        let current_pid = sysinfo::get_current_pid().expect("should get current pid");
        let name = get_process_name(&system, current_pid);
        assert!(name.is_some());
    }

    #[test]
    fn test_kill_processes_empty_list() {
        let system = create_system();
        let result = kill_processes(&system, &[]);
        assert_eq!(
            result,
            Err(WrathError::KillFailed(
                "Could not kill any processes".to_string()
            ))
        );
    }

    #[test]
    fn test_kill_processes_nonexistent_pids() {
        let system = create_system();
        let pids = vec![Pid::from_u32(999_999_999)];
        let result = kill_processes(&system, &pids);
        assert_eq!(
            result,
            Err(WrathError::KillFailed(
                "Could not kill any processes".to_string()
            ))
        );
    }
}
