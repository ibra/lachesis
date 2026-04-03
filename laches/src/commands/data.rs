use crate::db::{last_n_days_range, Database};
use std::error::Error;

/// Parse a duration string like "7d" or "30d" into a number of days.
pub fn parse_duration_days(s: &str) -> Result<i64, Box<dyn Error>> {
    if !s.ends_with('d') {
        return Err("error: duration must be in format like '7d', '30d'".into());
    }
    let num_str = &s[..s.len() - 1];
    let days: i64 = num_str
        .parse()
        .map_err(|_| "error: invalid duration value")?;
    if days <= 0 {
        return Err("error: duration must be a positive number".into());
    }
    Ok(days)
}

/// Export sessions to a JSON file.
pub fn export_sessions(
    db: &Database,
    output: &str,
    duration: Option<&str>,
) -> Result<(), Box<dyn Error>> {
    let sessions = if let Some(dur) = duration {
        let days = parse_duration_days(dur)?;
        let (start, end) = last_n_days_range(days);
        db.export_sessions(Some(&start), Some(&end))?
    } else {
        db.export_sessions(None, None)?
    };

    let json = serde_json::to_string_pretty(
        &sessions
            .iter()
            .map(|s| {
                serde_json::json!({
                    "process": s.process_name,
                    "exe_path": s.exe_path,
                    "window_title": s.window_title,
                    "start_time": s.start_time,
                    "end_time": s.end_time,
                    "idle": s.idle,
                })
            })
            .collect::<Vec<_>>(),
    )?;

    std::fs::write(output, &json)?;
    println!("exported {} sessions to '{}'", sessions.len(), output);
    Ok(())
}

/// Delete sessions by time range or all.
pub fn delete_sessions(
    db: &Database,
    all: bool,
    duration: Option<&str>,
) -> Result<(), Box<dyn Error>> {
    if all && duration.is_some() {
        return Err("error: cannot specify both --all and --duration".into());
    }
    if !all && duration.is_none() {
        return Err("error: must specify either --all or --duration".into());
    }

    if all {
        let count = db.delete_all_sessions()?;
        println!("deleted {} sessions", count);
    } else if let Some(dur) = duration {
        let days = parse_duration_days(dur)?;
        let (start, end) = last_n_days_range(days);
        let count = db.delete_sessions(&start, &end)?;
        println!("deleted {} sessions from the last {} days", count, days);
    }
    Ok(())
}

/// Reset all data (sessions and tags).
pub fn reset_data(db: &Database) -> Result<(), Box<dyn Error>> {
    db.reset()?;
    println!("all sessions and tags cleared.");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_duration_valid() {
        assert_eq!(parse_duration_days("7d").unwrap(), 7);
        assert_eq!(parse_duration_days("30d").unwrap(), 30);
        assert_eq!(parse_duration_days("365d").unwrap(), 365);
        assert_eq!(parse_duration_days("1d").unwrap(), 1);
    }

    #[test]
    fn test_parse_duration_missing_suffix() {
        assert!(parse_duration_days("7").is_err());
        assert!(parse_duration_days("30").is_err());
    }

    #[test]
    fn test_parse_duration_invalid_number() {
        assert!(parse_duration_days("abcd").is_err());
        assert!(parse_duration_days("d").is_err());
        assert!(parse_duration_days("7.5d").is_err());
    }

    #[test]
    fn test_parse_duration_zero_or_negative() {
        assert!(parse_duration_days("0d").is_err());
        assert!(parse_duration_days("-5d").is_err());
    }
}
