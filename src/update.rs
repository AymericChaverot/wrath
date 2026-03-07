use serde::Deserialize;

/// The GitHub repository for version checks.
const GITHUB_REPO: &str = "AymericChaverot/wrath";

/// The GitHub API URL for the latest release.
const GITHUB_API_URL: &str = "https://api.github.com/repos";

/// Response from the GitHub releases API (only fields we need).
#[derive(Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
}

/// Check if a newer version of wrath is available on GitHub.
///
/// Returns `Some(latest_version)` if a newer version exists,
/// or `None` if the current version is up to date or the check fails.
pub fn check_for_update(current_version: &str) -> Option<String> {
    let latest = fetch_latest_version().ok()?;
    let latest_clean = latest.strip_prefix('v').unwrap_or(&latest);
    let current_clean = current_version.strip_prefix('v').unwrap_or(current_version);

    if is_newer(latest_clean, current_clean) {
        Some(latest_clean.to_string())
    } else {
        None
    }
}

/// Fetch the latest release version tag from GitHub.
fn fetch_latest_version() -> Result<String, String> {
    let url = format!("{GITHUB_API_URL}/{GITHUB_REPO}/releases/latest");

    let mut response = ureq::get(&url)
        .header("User-Agent", "wrath-cli")
        .header("Accept", "application/vnd.github.v3+json")
        .call()
        .map_err(|e| format!("HTTP request failed: {e}"))?;

    let release: GitHubRelease = response
        .body_mut()
        .read_json()
        .map_err(|e| format!("Failed to parse response: {e}"))?;

    Ok(release.tag_name)
}

/// Compare two semver-like version strings.
/// Returns true if `latest` is newer than `current`.
pub fn is_newer(latest: &str, current: &str) -> bool {
    let parse = |v: &str| -> Vec<u32> {
        v.split('.')
            .filter_map(|part| part.parse::<u32>().ok())
            .collect()
    };

    let latest_parts = parse(latest);
    let current_parts = parse(current);

    for i in 0..latest_parts.len().max(current_parts.len()) {
        let l = latest_parts.get(i).copied().unwrap_or(0);
        let c = current_parts.get(i).copied().unwrap_or(0);
        if l > c {
            return true;
        }
        if l < c {
            return false;
        }
    }

    false
}

/// Format the update notification message.
pub fn update_message(current: &str, latest: &str) -> String {
    format!(
        "\n[i] Update available: {current} -> {latest}\n    Run the install script again to update, or visit:\n    https://github.com/{GITHUB_REPO}/releases/latest\n"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_newer_major() {
        assert!(is_newer("2.0.0", "1.0.0"));
    }

    #[test]
    fn test_is_newer_minor() {
        assert!(is_newer("1.1.0", "1.0.0"));
    }

    #[test]
    fn test_is_newer_patch() {
        assert!(is_newer("1.0.1", "1.0.0"));
    }

    #[test]
    fn test_is_not_newer_same() {
        assert!(!is_newer("1.0.0", "1.0.0"));
    }

    #[test]
    fn test_is_not_newer_older() {
        assert!(!is_newer("1.0.0", "2.0.0"));
    }

    #[test]
    fn test_is_newer_different_lengths() {
        assert!(is_newer("1.0.1", "1.0"));
    }

    #[test]
    fn test_is_not_newer_different_lengths_equal() {
        assert!(!is_newer("1.0", "1.0.0"));
    }

    #[test]
    fn test_is_newer_with_v_prefix_stripped() {
        // The caller should strip 'v' prefix before calling
        assert!(is_newer("0.2.0", "0.1.0"));
    }

    #[test]
    fn test_update_message_format() {
        let msg = update_message("0.1.0", "0.2.0");
        assert!(msg.contains("0.1.0"));
        assert!(msg.contains("0.2.0"));
        assert!(msg.contains("Update available"));
        assert!(msg.contains(GITHUB_REPO));
    }

    #[test]
    fn test_is_newer_empty_strings() {
        assert!(!is_newer("", ""));
    }

    #[test]
    fn test_is_newer_invalid_versions() {
        assert!(!is_newer("abc", "def"));
    }

    #[test]
    fn test_is_newer_mixed_valid_invalid() {
        // "1.abc" parses as [1], "0.1" parses as [0, 1]
        // Comparing: 1 > 0, so latest is newer
        assert!(is_newer("1.abc", "0.1"));
    }

    #[test]
    fn test_check_for_update_strips_v_prefix() {
        // This tests the prefix stripping logic without making network calls
        let latest = "v1.2.3";
        let current = "v1.2.2";
        let latest_clean = latest.strip_prefix('v').unwrap_or(latest);
        let current_clean = current.strip_prefix('v').unwrap_or(current);
        assert!(is_newer(latest_clean, current_clean));
    }
}
