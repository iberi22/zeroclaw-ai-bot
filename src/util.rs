//! Utility functions for `ZeroClaw`.
//!
//! This module contains reusable helper functions used across the codebase.

/// Truncate a string to at most `max_chars` characters, appending "..." if truncated.
///
/// This function safely handles multi-byte UTF-8 characters (emoji, CJK, accented characters)
/// by using character boundaries instead of byte indices.
///
/// # Arguments
/// * `s` - The string to truncate
/// * `max_chars` - Maximum number of characters to keep (excluding "...")
///
/// # Returns
/// * Original string if length <= `max_chars`
/// * Truncated string with "..." appended if length > `max_chars`
///
/// # Examples
/// ```
/// use zeroclaw::util::truncate_with_ellipsis;
///
/// // ASCII string - no truncation needed
/// assert_eq!(truncate_with_ellipsis("hello", 10), "hello");
///
/// // ASCII string - truncation needed
/// assert_eq!(truncate_with_ellipsis("hello world", 5), "hello...");
///
/// // Multi-byte UTF-8 (emoji) - safe truncation
/// assert_eq!(truncate_with_ellipsis("Hello 🦀 World", 8), "Hello 🦀...");
/// assert_eq!(truncate_with_ellipsis("😀😀😀😀", 2), "😀😀...");
///
/// // Empty string
/// assert_eq!(truncate_with_ellipsis("", 10), "");
/// ```
use std::path::PathBuf;

pub fn truncate_with_ellipsis(s: &str, max_chars: usize) -> String {
    match s.char_indices().nth(max_chars) {
        Some((idx, _)) => {
            let truncated = &s[..idx];
            // Trim trailing whitespace for cleaner output
            format!("{}...", truncated.trim_end())
        }
        None => s.to_string(),
    }
}

#[derive(Debug, Clone)]
pub struct OpenClawProfile {
    pub source_path: PathBuf,
    pub workspace_dir: Option<PathBuf>,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub api_url: Option<String>,
    pub api_key: Option<String>,
}

fn parse_openclaw_profile(config_path: &std::path::Path) -> Option<OpenClawProfile> {
    let content = std::fs::read_to_string(config_path).ok()?;
    let json: serde_json::Value = serde_json::from_str(&content).ok()?;

    let workspace_dir = json
        .get("agents")
        .and_then(|v| v.get("defaults"))
        .and_then(|v| v.get("workspace"))
        .and_then(serde_json::Value::as_str)
        .map(PathBuf::from);

    let primary = json
        .get("agents")
        .and_then(|v| v.get("defaults"))
        .and_then(|v| v.get("model"))
        .and_then(|v| v.get("primary"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or("");

    let (provider, model) = if let Some((p, m)) = primary.split_once('/') {
        (Some(p.to_string()), Some(m.to_string()))
    } else {
        (None, None)
    };

    let provider_node = provider
        .as_deref()
        .and_then(|p| json.get("models")?.get("providers")?.get(p));
    let api_url = provider_node
        .and_then(|v| v.get("baseUrl"))
        .and_then(serde_json::Value::as_str)
        .map(ToString::to_string);
    let api_key = provider_node
        .and_then(|v| v.get("apiKey"))
        .and_then(serde_json::Value::as_str)
        .map(ToString::to_string);

    Some(OpenClawProfile {
        source_path: config_path.to_path_buf(),
        workspace_dir,
        provider,
        model,
        api_url,
        api_key,
    })
}

/// Resolves the workspace directory specified in the OpenClaw configuration (`~/.openclaw/openclaw.json`), if available.
pub fn resolve_openclaw_workspace() -> Option<PathBuf> {
    detect_openclaw_profile().and_then(|profile| profile.workspace_dir)
}

/// Detect OpenClaw installation and extract provider/workspace defaults.
pub fn detect_openclaw_profile() -> Option<OpenClawProfile> {
    let home = directories::UserDirs::new()?.home_dir().to_path_buf();
    let candidates = [
        home.join(".openclaw").join("openclaw.json"),
        home.join(".clawdbot").join("openclaw.json"),
    ];

    for config_path in candidates {
        if !config_path.exists() {
            continue;
        }
        if let Some(profile) = parse_openclaw_profile(&config_path) {
            return Some(profile);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_ascii_no_truncation() {
        // ASCII string shorter than limit - no change
        assert_eq!(truncate_with_ellipsis("hello", 10), "hello");
        assert_eq!(truncate_with_ellipsis("hello world", 50), "hello world");
    }

    #[test]
    fn test_truncate_ascii_with_truncation() {
        // ASCII string longer than limit - truncates
        assert_eq!(truncate_with_ellipsis("hello world", 5), "hello...");
        assert_eq!(
            truncate_with_ellipsis("This is a long message", 10),
            "This is a..."
        );
    }

    #[test]
    fn test_truncate_empty_string() {
        assert_eq!(truncate_with_ellipsis("", 10), "");
    }

    #[test]
    fn test_truncate_at_exact_boundary() {
        // String exactly at boundary - no truncation
        assert_eq!(truncate_with_ellipsis("hello", 5), "hello");
    }

    #[test]
    fn test_truncate_emoji_single() {
        // Single emoji (4 bytes) - should not panic
        let s = "🦀";
        assert_eq!(truncate_with_ellipsis(s, 10), s);
        assert_eq!(truncate_with_ellipsis(s, 1), s);
    }

    #[test]
    fn test_truncate_emoji_multiple() {
        // Multiple emoji - safe truncation at character boundary
        let s = "😀😀😀😀"; // 4 emoji, each 4 bytes = 16 bytes total
        assert_eq!(truncate_with_ellipsis(s, 2), "😀😀...");
        assert_eq!(truncate_with_ellipsis(s, 3), "😀😀😀...");
    }

    #[test]
    fn test_truncate_mixed_ascii_emoji() {
        // Mixed ASCII and emoji
        assert_eq!(truncate_with_ellipsis("Hello 🦀 World", 8), "Hello 🦀...");
        assert_eq!(truncate_with_ellipsis("Hi 😊", 10), "Hi 😊");
    }

    #[test]
    fn test_truncate_cjk_characters() {
        // CJK characters (Chinese - each is 3 bytes)
        let s = "这是一个测试消息用来触发崩溃的中文"; // 21 characters
        let result = truncate_with_ellipsis(s, 16);
        assert!(result.ends_with("..."));
        assert!(result.is_char_boundary(result.len() - 1));
    }

    #[test]
    fn test_truncate_accented_characters() {
        // Accented characters (2 bytes each in UTF-8)
        let s = "café résumé naïve";
        assert_eq!(truncate_with_ellipsis(s, 10), "café résum...");
    }

    #[test]
    fn test_truncate_unicode_edge_case() {
        // Mix of 1-byte, 2-byte, 3-byte, and 4-byte characters
        let s = "aé你好🦀"; // 1 + 1 + 2 + 2 + 4 bytes = 10 bytes, 5 chars
        assert_eq!(truncate_with_ellipsis(s, 3), "aé你...");
    }

    #[test]
    fn test_truncate_long_string() {
        // Long ASCII string
        let s = "a".repeat(200);
        let result = truncate_with_ellipsis(&s, 50);
        assert_eq!(result.len(), 53); // 50 + "..."
        assert!(result.ends_with("..."));
    }

    #[test]
    fn test_truncate_zero_max_chars() {
        // Edge case: max_chars = 0
        assert_eq!(truncate_with_ellipsis("hello", 0), "...");
    }
}
