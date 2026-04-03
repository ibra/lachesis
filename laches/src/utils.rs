use std::io::{self, Write};

/// Truncate a string to at most `max_chars` characters, appending "..." if truncated.
/// Safe for multi-byte UTF-8 strings (never panics on char boundaries).
pub fn truncate_str(s: &str, max_chars: usize) -> String {
    let char_count = s.chars().count();
    if char_count <= max_chars {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_chars.saturating_sub(3)).collect();
        format!("{}...", truncated)
    }
}

/// Format a duration in seconds as a short string (hours, minutes, seconds).
/// Used by TUI views for session-level precision.
pub fn format_duration_short(seconds: i64) -> String {
    let seconds = seconds.max(0);
    let h = seconds / 3600;
    let m = (seconds % 3600) / 60;
    let s = seconds % 60;
    if h > 0 {
        format!("{}h {}m", h, m)
    } else if m > 0 {
        format!("{}m {}s", m, s)
    } else {
        format!("{}s", s)
    }
}

/// Format a duration in seconds as a compact hours+minutes string (no seconds).
/// Used for high-level summaries where seconds are noise.
pub fn format_duration_hm(seconds: i64) -> String {
    let seconds = seconds.max(0);
    let h = seconds / 3600;
    let m = (seconds % 3600) / 60;
    if h > 0 {
        format!("{}h {}m", h, m)
    } else {
        format!("{}m", m)
    }
}

pub fn format_uptime(seconds: u64) -> String {
    let days = seconds / 86400;
    let hours = (seconds % 86400) / 3600;
    let minutes = (seconds % 3600) / 60;
    let seconds = seconds % 60;

    if days > 0 {
        format!("{}d {}h {}m {}s", days, hours, minutes, seconds)
    } else if hours > 0 {
        format!("{}h {}m {}s", hours, minutes, seconds)
    } else if minutes > 0 {
        format!("{}m {}s", minutes, seconds)
    } else {
        format!("{}s", seconds)
    }
}

pub fn confirm(prompt: &str) -> bool {
    print!("{}", prompt);
    io::stdout().flush().unwrap();

    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();

    matches!(input.trim().to_lowercase().as_str(), "y" | "yes")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_uptime_seconds_only() {
        assert_eq!(format_uptime(0), "0s");
        assert_eq!(format_uptime(30), "30s");
        assert_eq!(format_uptime(59), "59s");
    }

    #[test]
    fn test_format_uptime_minutes() {
        assert_eq!(format_uptime(60), "1m 0s");
        assert_eq!(format_uptime(90), "1m 30s");
        assert_eq!(format_uptime(3599), "59m 59s");
    }

    #[test]
    fn test_format_uptime_hours() {
        assert_eq!(format_uptime(3600), "1h 0m 0s");
        assert_eq!(format_uptime(3661), "1h 1m 1s");
        assert_eq!(format_uptime(7200), "2h 0m 0s");
        assert_eq!(format_uptime(86399), "23h 59m 59s");
    }

    #[test]
    fn test_format_uptime_days() {
        assert_eq!(format_uptime(86400), "1d 0h 0m 0s");
        assert_eq!(format_uptime(90061), "1d 1h 1m 1s");
        assert_eq!(format_uptime(172800), "2d 0h 0m 0s");
        assert_eq!(format_uptime(259200), "3d 0h 0m 0s");
    }

    #[test]
    fn test_format_uptime_complex() {
        // 2 days, 5 hours, 30 minutes, 45 seconds
        let seconds = 2 * 86400 + 5 * 3600 + 30 * 60 + 45;
        assert_eq!(format_uptime(seconds), "2d 5h 30m 45s");
    }

    #[test]
    fn test_format_uptime_large_values() {
        // 365 days
        let one_year = 365 * 86400;
        assert_eq!(format_uptime(one_year), "365d 0h 0m 0s");

        // 100 days, 12 hours, 34 minutes, 56 seconds
        let complex = 100 * 86400 + 12 * 3600 + 34 * 60 + 56;
        assert_eq!(format_uptime(complex), "100d 12h 34m 56s");
    }

    #[test]
    fn test_truncate_str_short() {
        assert_eq!(truncate_str("hello", 10), "hello");
        assert_eq!(truncate_str("hello", 5), "hello");
    }

    #[test]
    fn test_truncate_str_exact_boundary() {
        assert_eq!(truncate_str("abcde", 5), "abcde");
    }

    #[test]
    fn test_truncate_str_long() {
        assert_eq!(truncate_str("hello world, this is long", 10), "hello w...");
    }

    #[test]
    fn test_truncate_str_multibyte_utf8() {
        // emoji and CJK characters are multi-byte in UTF-8
        let s = "日本語テスト文字列です";
        let result = truncate_str(s, 6);
        assert_eq!(result, "日本語...");
    }

    #[test]
    fn test_truncate_str_empty() {
        assert_eq!(truncate_str("", 5), "");
    }

    #[test]
    fn test_format_duration_short_seconds() {
        assert_eq!(format_duration_short(0), "0s");
        assert_eq!(format_duration_short(45), "45s");
    }

    #[test]
    fn test_format_duration_short_minutes() {
        assert_eq!(format_duration_short(90), "1m 30s");
        assert_eq!(format_duration_short(3599), "59m 59s");
    }

    #[test]
    fn test_format_duration_short_hours() {
        assert_eq!(format_duration_short(3600), "1h 0m");
        assert_eq!(format_duration_short(7260), "2h 1m");
    }

    #[test]
    fn test_format_duration_short_negative() {
        // negative values should be clamped to 0
        assert_eq!(format_duration_short(-100), "0s");
    }

    #[test]
    fn test_format_duration_hm_minutes() {
        assert_eq!(format_duration_hm(0), "0m");
        assert_eq!(format_duration_hm(90), "1m");
        assert_eq!(format_duration_hm(3599), "59m");
    }

    #[test]
    fn test_format_duration_hm_hours() {
        assert_eq!(format_duration_hm(3600), "1h 0m");
        assert_eq!(format_duration_hm(7260), "2h 1m");
    }
}
