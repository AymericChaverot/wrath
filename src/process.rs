use std::collections::{HashMap, HashSet, VecDeque};

use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System, Users};

use crate::error::WrathError;
use crate::inspect::ProcessInfo;

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
    /// First part of the message: which process, and how many were killed.
    pub fn headline(&self) -> String {
        let process_word = if self.total_killed == 1 {
            "process"
        } else {
            "processes"
        };

        format!(
            "Wrath unleashed: Process '{}' ({} {})",
            self.process_name, self.total_killed, process_word
        )
    }

    /// Second part of the message: what happened.
    pub fn outcome(&self) -> String {
        match self.port {
            Some(port) => format!("using port {port} has been obliterated."),
            None => "has been obliterated.".to_string(),
        }
    }
}

/// The set of processes wrath is about to kill.
#[derive(Debug, PartialEq)]
pub struct KillPlan {
    /// Name shown in the final message.
    pub process_name: String,
    /// The port being freed, if targeting a port.
    pub port: Option<u16>,
    /// Processes that matched the target (and are not descendants of another match).
    pub roots: Vec<Pid>,
    /// Every process to kill: roots first, then their descendants.
    pub pids: Vec<Pid>,
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

/// Plan the killing of every process with the given name, and their children.
pub fn plan_by_name(system: &System, name: &str) -> Result<KillPlan, WrathError> {
    let own_pid = sysinfo::get_current_pid().ok();
    let exe_name = format!("{name}.exe");
    let mut matching: Vec<Pid> = system
        .processes()
        .iter()
        .filter(|(pid, process)| {
            let proc_name = process.name().to_string_lossy();
            Some(**pid) != own_pid
                && (proc_name.eq_ignore_ascii_case(name)
                    || proc_name.eq_ignore_ascii_case(&exe_name))
        })
        .map(|(pid, _)| *pid)
        .collect();
    matching.sort();

    let Some(first) = matching.first() else {
        return Err(WrathError::NoProcessWithName(name.to_string()));
    };
    let process_name = get_process_name(system, *first).unwrap_or_else(|| name.to_string());

    let roots: Vec<Pid> = matching
        .iter()
        .copied()
        .filter(|pid| !has_ancestor_in(system, *pid, &matching))
        .collect();

    Ok(build_plan(system, process_name, None, roots))
}

/// Plan the killing of the process using the given port, and its children.
pub fn plan_by_pid(system: &System, pid: u32, port: u16) -> Result<KillPlan, WrathError> {
    let root = Pid::from_u32(pid);
    let process_name = get_process_name(system, root).ok_or(WrathError::NoProcessOnPort(port))?;

    Ok(build_plan(system, process_name, Some(port), vec![root]))
}

/// Describe every root of the plan, including the names of its descendants.
pub fn describe(system: &System, plan: &KillPlan) -> Vec<ProcessInfo> {
    let users = Users::new_with_refreshed_list();
    let children_map = children_map(system);

    plan.roots
        .iter()
        .filter_map(|pid| {
            let process = system.process(*pid)?;
            let children = descendants(&children_map, *pid)
                .into_iter()
                .filter_map(|child| get_process_name(system, child))
                .collect();
            Some(ProcessInfo::from_process(*pid, process, &users, children))
        })
        .collect()
}

/// Kill every process of the plan.
pub fn execute(system: &mut System, plan: &KillPlan) -> Result<KillResult, WrathError> {
    let total_killed = kill_processes(system, &plan.pids)?;

    Ok(KillResult {
        process_name: plan.process_name.clone(),
        total_killed,
        port: plan.port,
    })
}

/// Build a plan from its roots, collecting all of their descendants.
fn build_plan(
    system: &System,
    process_name: String,
    port: Option<u16>,
    roots: Vec<Pid>,
) -> KillPlan {
    let own_pid = sysinfo::get_current_pid().ok();
    let children_map = children_map(system);
    let mut seen: HashSet<Pid> = HashSet::new();

    // Roots first, so parents cannot respawn children that were already killed.
    let pids: Vec<Pid> = roots
        .iter()
        .copied()
        .chain(
            roots
                .iter()
                .flat_map(|root| descendants(&children_map, *root)),
        )
        .filter(|pid| Some(*pid) != own_pid && seen.insert(*pid))
        .collect();

    KillPlan {
        process_name,
        port,
        roots,
        pids,
    }
}

/// Get the name of a process by its PID.
fn get_process_name(system: &System, pid: Pid) -> Option<String> {
    system
        .process(pid)
        .map(|p| p.name().to_string_lossy().to_string())
}

/// Map every process to its direct children.
fn children_map(system: &System) -> HashMap<Pid, Vec<Pid>> {
    let mut map: HashMap<Pid, Vec<Pid>> = HashMap::new();
    for (pid, process) in system.processes() {
        if let Some(parent) = process.parent() {
            map.entry(parent).or_default().push(*pid);
        }
    }
    for children in map.values_mut() {
        children.sort();
    }
    map
}

/// Collect all descendants of `root`, breadth-first and cycle-safe.
fn descendants(children_map: &HashMap<Pid, Vec<Pid>>, root: Pid) -> Vec<Pid> {
    let mut seen: HashSet<Pid> = HashSet::from([root]);
    let mut result = Vec::new();
    let mut queue = VecDeque::from([root]);

    while let Some(pid) = queue.pop_front() {
        for child in children_map.get(&pid).into_iter().flatten() {
            if seen.insert(*child) {
                result.push(*child);
                queue.push_back(*child);
            }
        }
    }
    result
}

/// Whether one of the ancestors of `pid` is in `candidates`.
fn has_ancestor_in(system: &System, pid: Pid, candidates: &[Pid]) -> bool {
    let mut visited: HashSet<Pid> = HashSet::from([pid]);
    let mut current = system.process(pid).and_then(|p| p.parent());

    while let Some(parent) = current {
        if candidates.contains(&parent) {
            return true;
        }
        if !visited.insert(parent) {
            return false;
        }
        current = system.process(parent).and_then(|p| p.parent());
    }
    false
}

/// Kill all processes in the list. Returns how many of them are no longer running.
fn kill_processes(system: &mut System, pids: &[Pid]) -> Result<usize, WrathError> {
    let alive: Vec<Pid> = pids
        .iter()
        .copied()
        .filter(|pid| system.process(*pid).is_some())
        .collect();
    let failed: Vec<Pid> = alive
        .iter()
        .copied()
        .filter(|pid| !system.process(*pid).is_some_and(|p| p.kill()))
        .collect();

    // A failed kill often means the process already died along with its parent.
    if !failed.is_empty() {
        system.refresh_processes(ProcessesToUpdate::Some(&failed), true);
    }
    let survivors = failed
        .iter()
        .filter(|pid| system.process(**pid).is_some())
        .count();
    let killed = alive.len() - survivors;

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

    /// The whole result message on a single line.
    fn message(result: &KillResult) -> String {
        format!("{} {}", result.headline(), result.outcome())
    }

    fn pid(n: u32) -> Pid {
        Pid::from_u32(n)
    }

    #[test]
    fn test_kill_result_message_single_process_no_port() {
        let result = KillResult {
            process_name: "node".to_string(),
            total_killed: 1,
            port: None,
        };
        assert_eq!(
            message(&result),
            "Wrath unleashed: Process 'node' (1 process) has been obliterated."
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
            message(&result),
            "Wrath unleashed: Process 'node' (3 processes) has been obliterated."
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
            message(&result),
            "Wrath unleashed: Process 'node' (1 process) using port 3000 has been obliterated."
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
            message(&result),
            "Wrath unleashed: Process 'node' (5 processes) using port 8080 has been obliterated."
        );
    }

    #[test]
    fn test_create_system_returns_system() {
        let system = create_system();
        assert!(!system.processes().is_empty());
    }

    #[test]
    fn test_plan_by_name_nonexistent_process() {
        let system = create_system();
        let result = plan_by_name(&system, "this_process_definitely_does_not_exist_xyzzy_42");
        assert_eq!(
            result,
            Err(WrathError::NoProcessWithName(
                "this_process_definitely_does_not_exist_xyzzy_42".to_string()
            ))
        );
    }

    #[test]
    fn test_plan_by_name_never_targets_itself() {
        let system = create_system();
        let own_pid = sysinfo::get_current_pid().expect("should get current pid");
        let own_name = get_process_name(&system, own_pid).expect("current process has a name");

        if let Ok(plan) = plan_by_name(&system, &own_name) {
            assert!(!plan.pids.contains(&own_pid));
        }
    }

    #[test]
    fn test_plan_by_pid_nonexistent_process() {
        let system = create_system();
        let result = plan_by_pid(&system, 999_999_999, 3000);
        assert_eq!(result, Err(WrathError::NoProcessOnPort(3000)));
    }

    #[test]
    fn test_plan_by_pid_excludes_current_process() {
        let system = create_system();
        let own_pid = sysinfo::get_current_pid().expect("should get current pid");

        let plan = plan_by_pid(&system, own_pid.as_u32(), 3000).expect("current process exists");
        assert_eq!(plan.roots, vec![own_pid]);
        assert_eq!(plan.port, Some(3000));
        assert!(!plan.pids.contains(&own_pid));
    }

    #[test]
    fn test_describe_current_process() {
        let system = create_system();
        let own_pid = sysinfo::get_current_pid().expect("should get current pid");
        let plan = plan_by_pid(&system, own_pid.as_u32(), 3000).expect("current process exists");

        let infos = describe(&system, &plan);
        assert_eq!(infos.len(), 1);
        assert_eq!(infos[0].pid, own_pid.as_u32());
    }

    #[test]
    fn test_execute_empty_plan_fails() {
        let mut system = create_system();
        let plan = KillPlan {
            process_name: "ghost".to_string(),
            port: None,
            roots: vec![],
            pids: vec![],
        };
        assert!(matches!(
            execute(&mut system, &plan),
            Err(WrathError::KillFailed(_))
        ));
    }

    #[test]
    fn test_descendants_breadth_first() {
        let map = HashMap::from([
            (pid(1), vec![pid(2), pid(3)]),
            (pid(2), vec![pid(4)]),
            (pid(3), vec![pid(5)]),
        ]);
        assert_eq!(
            descendants(&map, pid(1)),
            vec![pid(2), pid(3), pid(4), pid(5)]
        );
        assert_eq!(descendants(&map, pid(3)), vec![pid(5)]);
        assert!(descendants(&map, pid(5)).is_empty());
    }

    #[test]
    fn test_descendants_survives_cycles() {
        let map = HashMap::from([(pid(1), vec![pid(2)]), (pid(2), vec![pid(1)])]);
        assert_eq!(descendants(&map, pid(1)), vec![pid(2)]);
    }

    #[test]
    fn test_has_ancestor_in_unknown_pid() {
        let system = create_system();
        assert!(!has_ancestor_in(&system, pid(999_999_999), &[pid(1)]));
    }

    #[test]
    fn test_get_process_name_nonexistent() {
        let system = create_system();
        assert!(get_process_name(&system, pid(999_999_999)).is_none());
    }

    #[test]
    fn test_get_process_name_current_process() {
        let system = create_system();
        let current_pid = sysinfo::get_current_pid().expect("should get current pid");
        assert!(get_process_name(&system, current_pid).is_some());
    }

    #[test]
    fn test_kill_processes_empty_list() {
        let mut system = create_system();
        assert_eq!(
            kill_processes(&mut system, &[]),
            Err(WrathError::KillFailed(
                "Could not kill any processes".to_string()
            ))
        );
    }

    #[test]
    fn test_kill_processes_nonexistent_pids() {
        let mut system = create_system();
        assert_eq!(
            kill_processes(&mut system, &[pid(999_999_999)]),
            Err(WrathError::KillFailed(
                "Could not kill any processes".to_string()
            ))
        );
    }
}
