use std::io::{self, Write};

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
}
