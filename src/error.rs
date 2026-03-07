use std::fmt;

/// Errors that can occur during wrath operations.
#[derive(Debug, PartialEq)]
pub enum WrathError {
    /// No process found using the specified port.
    NoProcessOnPort(u16),
    /// No process found with the specified name.
    NoProcessWithName(String),
    /// Failed to resolve port to a process ID.
    PortResolutionFailed(u16, String),
    /// Failed to kill a process.
    KillFailed(String),
}

impl fmt::Display for WrathError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WrathError::NoProcessOnPort(port) => {
                write!(f, "No process found using port {port}")
            }
            WrathError::NoProcessWithName(name) => {
                write!(f, "No process found with name '{name}'")
            }
            WrathError::PortResolutionFailed(port, reason) => {
                write!(f, "Failed to resolve port {port}: {reason}")
            }
            WrathError::KillFailed(reason) => {
                write!(f, "Failed to kill process: {reason}")
            }
        }
    }
}

impl std::error::Error for WrathError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_process_on_port_display() {
        let err = WrathError::NoProcessOnPort(3000);
        assert_eq!(err.to_string(), "No process found using port 3000");
    }

    #[test]
    fn test_no_process_with_name_display() {
        let err = WrathError::NoProcessWithName("node".to_string());
        assert_eq!(err.to_string(), "No process found with name 'node'");
    }

    #[test]
    fn test_port_resolution_failed_display() {
        let err = WrathError::PortResolutionFailed(8080, "timeout".to_string());
        assert_eq!(err.to_string(), "Failed to resolve port 8080: timeout");
    }

    #[test]
    fn test_kill_failed_display() {
        let err = WrathError::KillFailed("permission denied".to_string());
        assert_eq!(err.to_string(), "Failed to kill process: permission denied");
    }

    #[test]
    fn test_error_debug() {
        let err = WrathError::NoProcessOnPort(80);
        let debug_str = format!("{err:?}");
        assert!(debug_str.contains("NoProcessOnPort"));
    }

    #[test]
    fn test_error_equality() {
        assert_eq!(
            WrathError::NoProcessOnPort(80),
            WrathError::NoProcessOnPort(80)
        );
        assert_ne!(
            WrathError::NoProcessOnPort(80),
            WrathError::NoProcessOnPort(81)
        );
    }
}
