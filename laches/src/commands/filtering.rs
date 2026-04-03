use regex::Regex;

/// Check if a process name matches any pattern in the list.
/// Supports both exact matches and regex. On windows, exact matches
/// are case-insensitive since process names are case-insensitive there.
pub fn matches_any_pattern(process_name: &str, patterns: &[String]) -> bool {
    for pattern in patterns {
        if cfg!(windows) {
            if pattern.eq_ignore_ascii_case(process_name) {
                return true;
            }
        } else if pattern == process_name {
            return true;
        }

        if let Ok(regex) = Regex::new(pattern) {
            if regex.is_match(process_name) {
                return true;
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_matches_any_pattern_exact_match() {
        let patterns = vec![
            "chrome.exe".to_string(),
            "firefox.exe".to_string(),
            "notepad.exe".to_string(),
        ];

        assert!(matches_any_pattern("chrome.exe", &patterns));
        assert!(matches_any_pattern("firefox.exe", &patterns));
        assert!(matches_any_pattern("notepad.exe", &patterns));
        assert!(!matches_any_pattern("explorer.exe", &patterns));
    }

    #[test]
    fn test_matches_any_pattern_regex() {
        let patterns = vec![".*chrom.*".to_string(), "^notepad.*".to_string()];

        assert!(matches_any_pattern("chrome", &patterns));
        assert!(matches_any_pattern("google-chrome", &patterns));
        assert!(matches_any_pattern("chromium", &patterns));
        assert!(matches_any_pattern("notepad.exe", &patterns));
        assert!(matches_any_pattern("notepad++", &patterns));
        assert!(!matches_any_pattern("firefox", &patterns));
    }

    #[test]
    fn test_matches_any_pattern_mixed() {
        let patterns = vec!["chrome.exe".to_string(), ".*firefox.*".to_string()];

        assert!(matches_any_pattern("chrome.exe", &patterns));
        assert!(matches_any_pattern("firefox", &patterns));
        assert!(matches_any_pattern("mozilla-firefox", &patterns));
        assert!(!matches_any_pattern("chrome", &patterns));
    }

    #[test]
    fn test_matches_any_pattern_empty() {
        let patterns: Vec<String> = vec![];
        assert!(!matches_any_pattern("anything", &patterns));
    }

    #[test]
    fn test_matches_any_pattern_invalid_regex() {
        let patterns = vec!["[invalid".to_string(), "valid.exe".to_string()];

        assert!(!matches_any_pattern("invalid", &patterns));
        assert!(matches_any_pattern("valid.exe", &patterns));
    }

    #[test]
    fn test_matches_any_pattern_case_handling() {
        let patterns = vec!["Chrome.exe".to_string()];

        assert!(matches_any_pattern("Chrome.exe", &patterns));
        if cfg!(windows) {
            assert!(matches_any_pattern("chrome.exe", &patterns));
        } else {
            assert!(!matches_any_pattern("chrome.exe", &patterns));
        }
    }

    #[test]
    fn test_matches_any_pattern_complex_regex() {
        let patterns = vec![
            r"^(chrome|firefox|edge)\.exe$".to_string(),
            r"\d+".to_string(),
        ];

        assert!(matches_any_pattern("chrome.exe", &patterns));
        assert!(matches_any_pattern("firefox.exe", &patterns));
        assert!(matches_any_pattern("edge.exe", &patterns));
        assert!(matches_any_pattern("test123", &patterns));
        assert!(!matches_any_pattern("safari.exe", &patterns));
        assert!(!matches_any_pattern("nodigits", &patterns));
    }
}
