use crate::config::FilterPattern;
use regex::Regex;

/// Check if a process name matches any pattern in the list.
/// Exact patterns use string comparison (case-insensitive on Windows).
/// Regex patterns are only applied when explicitly marked as regex.
pub fn matches_any_pattern(process_name: &str, patterns: &[FilterPattern]) -> bool {
    for fp in patterns {
        if fp.is_regex {
            if let Ok(regex) = Regex::new(&fp.pattern) {
                if regex.is_match(process_name) {
                    return true;
                }
            }
        } else if cfg!(windows) {
            if fp.pattern.eq_ignore_ascii_case(process_name) {
                return true;
            }
        } else if fp.pattern == process_name {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exact_match() {
        let patterns = vec![
            FilterPattern::exact("chrome.exe"),
            FilterPattern::exact("firefox.exe"),
            FilterPattern::exact("notepad.exe"),
        ];

        assert!(matches_any_pattern("chrome.exe", &patterns));
        assert!(matches_any_pattern("firefox.exe", &patterns));
        assert!(matches_any_pattern("notepad.exe", &patterns));
        assert!(!matches_any_pattern("explorer.exe", &patterns));
    }

    #[test]
    fn test_exact_match_does_not_regex() {
        // "chrome.exe" as an exact pattern should NOT match "chromeXexe"
        // because the dot is not treated as a regex wildcard
        let patterns = vec![FilterPattern::exact("chrome.exe")];

        assert!(matches_any_pattern("chrome.exe", &patterns));
        assert!(!matches_any_pattern("chromeXexe", &patterns));
    }

    #[test]
    fn test_regex_match() {
        let patterns = vec![
            FilterPattern::regex(".*chrom.*"),
            FilterPattern::regex("^notepad.*"),
        ];

        assert!(matches_any_pattern("chrome", &patterns));
        assert!(matches_any_pattern("google-chrome", &patterns));
        assert!(matches_any_pattern("chromium", &patterns));
        assert!(matches_any_pattern("notepad.exe", &patterns));
        assert!(matches_any_pattern("notepad++", &patterns));
        assert!(!matches_any_pattern("firefox", &patterns));
    }

    #[test]
    fn test_mixed_exact_and_regex() {
        let patterns = vec![
            FilterPattern::exact("chrome.exe"),
            FilterPattern::regex(".*firefox.*"),
        ];

        assert!(matches_any_pattern("chrome.exe", &patterns));
        assert!(matches_any_pattern("firefox", &patterns));
        assert!(matches_any_pattern("mozilla-firefox", &patterns));
        // exact "chrome.exe" should NOT match plain "chrome"
        assert!(!matches_any_pattern("chrome", &patterns));
    }

    #[test]
    fn test_empty_patterns() {
        let patterns: Vec<FilterPattern> = vec![];
        assert!(!matches_any_pattern("anything", &patterns));
    }

    #[test]
    fn test_invalid_regex_is_skipped() {
        let patterns = vec![
            FilterPattern::regex("[invalid"),
            FilterPattern::exact("valid.exe"),
        ];

        assert!(!matches_any_pattern("invalid", &patterns));
        assert!(matches_any_pattern("valid.exe", &patterns));
    }

    #[test]
    fn test_case_handling() {
        let patterns = vec![FilterPattern::exact("Chrome.exe")];

        assert!(matches_any_pattern("Chrome.exe", &patterns));
        if cfg!(windows) {
            assert!(matches_any_pattern("chrome.exe", &patterns));
        } else {
            assert!(!matches_any_pattern("chrome.exe", &patterns));
        }
    }

    #[test]
    fn test_complex_regex() {
        let patterns = vec![
            FilterPattern::regex(r"^(chrome|firefox|edge)\.exe$"),
            FilterPattern::regex(r"\d+"),
        ];

        assert!(matches_any_pattern("chrome.exe", &patterns));
        assert!(matches_any_pattern("firefox.exe", &patterns));
        assert!(matches_any_pattern("edge.exe", &patterns));
        assert!(matches_any_pattern("test123", &patterns));
        assert!(!matches_any_pattern("safari.exe", &patterns));
        assert!(!matches_any_pattern("nodigits", &patterns));
    }
}
