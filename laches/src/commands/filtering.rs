use crate::config::{FilterMode, FilterPattern};
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

enum CompiledPattern {
    Exact(String),
    Regex(Regex),
}

impl CompiledPattern {
    fn matches(&self, name: &str) -> bool {
        match self {
            CompiledPattern::Exact(s) => {
                if cfg!(windows) {
                    s.eq_ignore_ascii_case(name)
                } else {
                    s == name
                }
            }
            CompiledPattern::Regex(r) => r.is_match(name),
        }
    }
}

pub struct CompiledFilter {
    mode: FilterMode,
    whitelist: Vec<CompiledPattern>,
    blacklist: Vec<CompiledPattern>,
}

impl CompiledFilter {
    pub fn new(mode: FilterMode, whitelist: &[FilterPattern], blacklist: &[FilterPattern]) -> Self {
        Self {
            mode,
            whitelist: Self::compile_patterns(whitelist),
            blacklist: Self::compile_patterns(blacklist),
        }
    }

    fn compile_patterns(patterns: &[FilterPattern]) -> Vec<CompiledPattern> {
        patterns
            .iter()
            .filter_map(|p| {
                if p.is_regex {
                    Regex::new(&p.pattern).ok().map(CompiledPattern::Regex)
                } else {
                    Some(CompiledPattern::Exact(p.pattern.clone()))
                }
            })
            .collect()
    }

    pub fn should_track(&self, process_name: &str) -> bool {
        match self.mode {
            FilterMode::Default => true,
            FilterMode::Whitelist => self.whitelist.iter().any(|p| p.matches(process_name)),
            FilterMode::Blacklist => !self.blacklist.iter().any(|p| p.matches(process_name)),
        }
    }
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

    #[test]
    fn test_compiled_filter_default() {
        let f = CompiledFilter::new(FilterMode::Default, &[], &[]);
        assert!(f.should_track("anything"));
    }

    #[test]
    fn test_compiled_filter_whitelist() {
        let wl = vec![
            FilterPattern::exact("firefox"),
            FilterPattern::regex("^chrom.*"),
        ];
        let f = CompiledFilter::new(FilterMode::Whitelist, &wl, &[]);
        assert!(f.should_track("firefox"));
        assert!(f.should_track("chrome"));
        assert!(f.should_track("chromium"));
        assert!(!f.should_track("discord"));
    }

    #[test]
    fn test_compiled_filter_blacklist() {
        let bl = vec![FilterPattern::exact("discord")];
        let f = CompiledFilter::new(FilterMode::Blacklist, &[], &bl);
        assert!(!f.should_track("discord"));
        assert!(f.should_track("firefox"));
    }

    #[test]
    fn test_compiled_filter_skips_invalid_regex() {
        let wl = vec![
            FilterPattern::regex("[invalid"),
            FilterPattern::exact("valid"),
        ];
        let f = CompiledFilter::new(FilterMode::Whitelist, &wl, &[]);
        assert!(f.should_track("valid"));
        assert!(!f.should_track("invalid"));
    }
}
