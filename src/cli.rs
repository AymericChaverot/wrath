use clap::Parser;

/// Wrath CLI - Annihilate any process by name or port.
#[derive(Parser, Debug)]
#[command(
    name = "wrath",
    version,
    about = "Annihilate any process by name or port"
)]
pub struct Cli {
    /// The target to kill: a port number or a process name.
    #[arg(required = true)]
    pub target: String,
}

/// Represents the parsed target from the user input.
#[derive(Debug, PartialEq)]
pub enum Target {
    /// Kill process(es) using a specific port.
    Port(u16),
    /// Kill process(es) by name.
    Name(String),
}

/// Parse the raw target string into a structured `Target`.
///
/// If the target string can be parsed as a valid port number (1-65535),
/// it is treated as a port. Otherwise, it is treated as a process name.
pub fn parse_target(target: &str) -> Target {
    match target.parse::<u16>() {
        Ok(port) if port > 0 => Target::Port(port),
        _ => Target::Name(target.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_target_port() {
        assert_eq!(parse_target("3000"), Target::Port(3000));
    }

    #[test]
    fn test_parse_target_port_min() {
        assert_eq!(parse_target("1"), Target::Port(1));
    }

    #[test]
    fn test_parse_target_port_max() {
        assert_eq!(parse_target("65535"), Target::Port(65535));
    }

    #[test]
    fn test_parse_target_zero_is_name() {
        assert_eq!(parse_target("0"), Target::Name("0".to_string()));
    }

    #[test]
    fn test_parse_target_name() {
        assert_eq!(parse_target("node"), Target::Name("node".to_string()));
    }

    #[test]
    fn test_parse_target_name_with_dots() {
        assert_eq!(
            parse_target("my.process"),
            Target::Name("my.process".to_string())
        );
    }

    #[test]
    fn test_parse_target_name_with_dashes() {
        assert_eq!(
            parse_target("my-process"),
            Target::Name("my-process".to_string())
        );
    }

    #[test]
    fn test_parse_target_large_number_is_name() {
        // 70000 exceeds u16 range, so it should be treated as a name
        assert_eq!(parse_target("70000"), Target::Name("70000".to_string()));
    }

    #[test]
    fn test_parse_target_negative_is_name() {
        assert_eq!(parse_target("-1"), Target::Name("-1".to_string()));
    }

    #[test]
    fn test_parse_target_empty_is_name() {
        assert_eq!(parse_target(""), Target::Name(String::new()));
    }

    #[test]
    fn test_parse_target_mixed_input() {
        assert_eq!(
            parse_target("node3000"),
            Target::Name("node3000".to_string())
        );
    }

    #[test]
    fn test_cli_debug() {
        let cli = Cli {
            target: "3000".to_string(),
        };
        let debug_str = format!("{cli:?}");
        assert!(debug_str.contains("3000"));
    }

    #[test]
    fn test_target_equality() {
        assert_eq!(Target::Port(80), Target::Port(80));
        assert_ne!(Target::Port(80), Target::Port(81));
        assert_ne!(Target::Port(80), Target::Name("80".to_string()));
    }
}
