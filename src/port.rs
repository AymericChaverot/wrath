use std::process::Command;

use crate::error::WrathError;

/// Resolve a port number to the PID of the process using it.
///
/// This function uses platform-specific commands to find which process
/// is listening on the given port.
pub fn resolve_port_to_pid(port: u16) -> Result<u32, WrathError> {
    let output = run_netstat_command(port)?;
    parse_pid_from_output(&output, port)
}

/// Run the platform-specific command to find the PID using a port.
fn run_netstat_command(port: u16) -> Result<String, WrathError> {
    let result = if cfg!(target_os = "windows") {
        Command::new("cmd")
            .args(["/C", &format!("netstat -ano | findstr :{port}")])
            .output()
    } else {
        Command::new("sh")
            .args(["-c", &format!("lsof -i :{port} -t")])
            .output()
    };

    match result {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            if stdout.trim().is_empty() {
                Err(WrathError::NoProcessOnPort(port))
            } else {
                Ok(stdout)
            }
        }
        Err(e) => Err(WrathError::PortResolutionFailed(port, e.to_string())),
    }
}

/// Parse the PID from command output.
///
/// On Windows, netstat output looks like:
///   TCP    0.0.0.0:3000    0.0.0.0:0    LISTENING    12345
/// On Unix, lsof -t output is just the PID per line.
pub fn parse_pid_from_output(output: &str, port: u16) -> Result<u32, WrathError> {
    if cfg!(target_os = "windows") {
        parse_windows_netstat(output, port)
    } else {
        parse_unix_lsof(output, port)
    }
}

/// Parse PID from Windows netstat output.
///
/// Looks for LISTENING lines and extracts the last column (PID).
fn parse_windows_netstat(output: &str, port: u16) -> Result<u32, WrathError> {
    for line in output.lines() {
        let line = line.trim();
        if line.contains("LISTENING") {
            if let Some(pid_str) = line.split_whitespace().last() {
                if let Ok(pid) = pid_str.parse::<u32>() {
                    if pid > 0 {
                        return Ok(pid);
                    }
                }
            }
        }
    }
    Err(WrathError::NoProcessOnPort(port))
}

/// Parse PID from Unix lsof -t output.
///
/// Each line is a PID. We take the first one.
fn parse_unix_lsof(output: &str, port: u16) -> Result<u32, WrathError> {
    for line in output.lines() {
        let line = line.trim();
        if let Ok(pid) = line.parse::<u32>() {
            if pid > 0 {
                return Ok(pid);
            }
        }
    }
    Err(WrathError::NoProcessOnPort(port))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_windows_netstat_listening() {
        let output =
            "  TCP    0.0.0.0:3000           0.0.0.0:0              LISTENING       12345\r\n";
        let pid = parse_windows_netstat(output, 3000);
        assert_eq!(pid, Ok(12345));
    }

    #[test]
    fn test_parse_windows_netstat_multiple_lines() {
        let output = "  TCP    0.0.0.0:3000           0.0.0.0:0              ESTABLISHED     99999\r\n\
                       TCP    0.0.0.0:3000           0.0.0.0:0              LISTENING       12345\r\n";
        let pid = parse_windows_netstat(output, 3000);
        assert_eq!(pid, Ok(12345));
    }

    #[test]
    fn test_parse_windows_netstat_no_listening() {
        let output =
            "  TCP    0.0.0.0:3000           0.0.0.0:0              ESTABLISHED     12345\r\n";
        let pid = parse_windows_netstat(output, 3000);
        assert_eq!(pid, Err(WrathError::NoProcessOnPort(3000)));
    }

    #[test]
    fn test_parse_windows_netstat_empty() {
        let pid = parse_windows_netstat("", 3000);
        assert_eq!(pid, Err(WrathError::NoProcessOnPort(3000)));
    }

    #[test]
    fn test_parse_windows_netstat_pid_zero() {
        let output = "  TCP    0.0.0.0:3000           0.0.0.0:0              LISTENING       0\r\n";
        let pid = parse_windows_netstat(output, 3000);
        assert_eq!(pid, Err(WrathError::NoProcessOnPort(3000)));
    }

    #[test]
    fn test_parse_unix_lsof_single_pid() {
        let output = "12345\n";
        let pid = parse_unix_lsof(output, 3000);
        assert_eq!(pid, Ok(12345));
    }

    #[test]
    fn test_parse_unix_lsof_multiple_pids() {
        let output = "12345\n67890\n";
        let pid = parse_unix_lsof(output, 3000);
        assert_eq!(pid, Ok(12345));
    }

    #[test]
    fn test_parse_unix_lsof_empty() {
        let pid = parse_unix_lsof("", 3000);
        assert_eq!(pid, Err(WrathError::NoProcessOnPort(3000)));
    }

    #[test]
    fn test_parse_unix_lsof_invalid_output() {
        let pid = parse_unix_lsof("not_a_number\n", 3000);
        assert_eq!(pid, Err(WrathError::NoProcessOnPort(3000)));
    }

    #[test]
    fn test_parse_unix_lsof_pid_zero() {
        let pid = parse_unix_lsof("0\n", 3000);
        assert_eq!(pid, Err(WrathError::NoProcessOnPort(3000)));
    }

    #[test]
    fn test_parse_pid_from_output_dispatches_correctly() {
        // On the current platform, one of these branches will be tested
        if cfg!(target_os = "windows") {
            let output =
                "  TCP    0.0.0.0:8080           0.0.0.0:0              LISTENING       5678\r\n";
            assert_eq!(parse_pid_from_output(output, 8080), Ok(5678));
        } else {
            let output = "5678\n";
            assert_eq!(parse_pid_from_output(output, 8080), Ok(5678));
        }
    }

    #[test]
    fn test_parse_windows_netstat_whitespace_variations() {
        let output = "TCP    127.0.0.1:3000    0.0.0.0:0    LISTENING    42\r\n";
        let pid = parse_windows_netstat(output, 3000);
        assert_eq!(pid, Ok(42));
    }
}
