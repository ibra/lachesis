use crate::db::{date_range_for_day, last_n_days_range, today_range, Database};
use crate::utils::format_uptime;
use colored::Colorize;
use std::error::Error;

/// Resolve CLI time-range flags into (start, end, label) strings.
pub fn resolve_time_range(
    today: bool,
    week: bool,
    month: bool,
    date: Option<&str>,
    range: Option<&str>,
) -> Result<(String, String, String), Box<dyn Error>> {
    if let Some(r) = range {
        let parts: Vec<&str> = r.split("..").collect();
        if parts.len() != 2 {
            return Err("error: range must be YYYY-MM-DD..YYYY-MM-DD".into());
        }
        let (s, _) = date_range_for_day(parts[0]).ok_or("error: invalid start date in range")?;
        let (_, e) = date_range_for_day(parts[1]).ok_or("error: invalid end date in range")?;
        let label = format!("{} to {}", parts[0], parts[1]);
        return Ok((s, e, label));
    }

    if let Some(d) = date {
        let (s, e) = date_range_for_day(d).ok_or("error: invalid date format, use YYYY-MM-DD")?;
        return Ok((s, e, format!("usage for {}", d)));
    }

    if today {
        let (s, e) = today_range();
        return Ok((s, e, "today's usage".to_string()));
    }

    if week {
        let (s, e) = last_n_days_range(7);
        return Ok((s, e, "last 7 days".to_string()));
    }

    if month {
        let (s, e) = last_n_days_range(30);
        return Ok((s, e, "last 30 days".to_string()));
    }

    // default: all time
    let (s, e) = last_n_days_range(365 * 10);
    Ok((s, e, "all time".to_string()))
}

/// Print per-process usage summaries for a time range.
pub fn print_process_summaries(
    db: &Database,
    start: &str,
    end: &str,
    label: &str,
    tag_filter: Option<&str>,
    verbose: bool,
) -> Result<(), Box<dyn Error>> {
    let summaries = db.query_process_summaries(start, end, tag_filter)?;

    if summaries.is_empty() {
        println!("no tracked data for this period.");
        return Ok(());
    }

    let total: i64 = summaries.iter().map(|s| s.total_seconds).sum();
    let max_seconds = summaries.iter().map(|s| s.total_seconds).max().unwrap_or(1);

    let header = if let Some(t) = tag_filter {
        format!("{} (tag: {})", label, t)
    } else {
        label.to_string()
    };
    println!("{}", header.bold().cyan());
    println!();

    for (i, s) in summaries.iter().enumerate() {
        let bar_width: usize = 20;
        let filled = ((s.total_seconds as f64 / max_seconds as f64) * bar_width as f64) as usize;
        let empty = bar_width.saturating_sub(filled);
        let bar = format!(
            "{}{}",
            "#".repeat(filled).green(),
            ".".repeat(empty).dimmed()
        );

        let pct = if total > 0 {
            (s.total_seconds as f64 / total as f64 * 100.0) as u32
        } else {
            0
        };

        let tags = db.get_tags(&s.process_name).unwrap_or_default();
        let tag_str = if tags.is_empty() {
            String::new()
        } else {
            format!(" [{}]", tags.join(", "))
        };

        if verbose {
            println!(
                "  {:>2}. {:<22} {:>10}  {} {:>3}%  {:>2}d avg:{:>8}  {}sess{}",
                i + 1,
                s.process_name,
                format_uptime(s.total_seconds as u64),
                bar,
                pct,
                s.active_days,
                format_uptime(if s.active_days > 0 {
                    s.total_seconds as u64 / s.active_days as u64
                } else {
                    0
                }),
                s.session_count,
                tag_str.dimmed(),
            );
        } else {
            println!(
                "  {:>2}. {:<22} {:>10}  {} {:>3}%{}",
                i + 1,
                s.process_name,
                format_uptime(s.total_seconds as u64),
                bar,
                pct,
                tag_str.dimmed(),
            );
        }
    }

    println!();
    println!(
        "  {} processes, {} total",
        summaries.len(),
        format_uptime(total as u64)
    );

    Ok(())
}

/// Print individual sessions for a time range.
pub fn print_sessions(
    db: &Database,
    start: &str,
    end: &str,
    label: &str,
) -> Result<(), Box<dyn Error>> {
    let sessions = db.query_sessions(start, end)?;

    if sessions.is_empty() {
        println!("no sessions for this period.");
        return Ok(());
    }

    println!("{}", format!("sessions - {}", label).bold().cyan());
    println!();

    for s in &sessions {
        if s.idle {
            continue;
        }

        let start_short = s.start_time.get(11..16).unwrap_or(&s.start_time);
        let end_short = s
            .end_time
            .as_ref()
            .and_then(|e| e.get(11..16))
            .unwrap_or("now");

        let duration = if let Some(ref et) = s.end_time {
            let st =
                chrono::NaiveDateTime::parse_from_str(&s.start_time, crate::db::TIMESTAMP_FORMAT);
            let en = chrono::NaiveDateTime::parse_from_str(et, crate::db::TIMESTAMP_FORMAT);
            if let (Ok(st), Ok(en)) = (st, en) {
                let secs = (en - st).num_seconds().max(0) as u64;
                format_uptime(secs)
            } else {
                "?".to_string()
            }
        } else {
            "active".to_string()
        };

        let title = s.window_title.as_deref().unwrap_or("");
        let title_display = crate::utils::truncate_str(title, 40);

        println!(
            "  {}-{}  {:<22} {:>8}  {}",
            start_short,
            end_short,
            s.process_name,
            duration,
            title_display.dimmed(),
        );
    }

    println!();
    println!("  {} sessions", sessions.len());

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_today() {
        let (s, e, label) = resolve_time_range(true, false, false, None, None).unwrap();
        assert!(s.ends_with("T00:00:00"));
        assert!(e.ends_with("T00:00:00"));
        assert_eq!(label, "today's usage");
    }

    #[test]
    fn test_resolve_week() {
        let (_, _, label) = resolve_time_range(false, true, false, None, None).unwrap();
        assert_eq!(label, "last 7 days");
    }

    #[test]
    fn test_resolve_month() {
        let (_, _, label) = resolve_time_range(false, false, true, None, None).unwrap();
        assert_eq!(label, "last 30 days");
    }

    #[test]
    fn test_resolve_specific_date() {
        let (s, e, label) =
            resolve_time_range(false, false, false, Some("2026-04-01"), None).unwrap();
        assert_eq!(s, "2026-04-01T00:00:00");
        assert_eq!(e, "2026-04-02T00:00:00");
        assert_eq!(label, "usage for 2026-04-01");
    }

    #[test]
    fn test_resolve_date_range() {
        let (s, e, label) =
            resolve_time_range(false, false, false, None, Some("2026-04-01..2026-04-03")).unwrap();
        assert_eq!(s, "2026-04-01T00:00:00");
        assert_eq!(e, "2026-04-04T00:00:00");
        assert_eq!(label, "2026-04-01 to 2026-04-03");
    }

    #[test]
    fn test_resolve_default_all_time() {
        let (_, _, label) = resolve_time_range(false, false, false, None, None).unwrap();
        assert_eq!(label, "all time");
    }

    #[test]
    fn test_resolve_invalid_date() {
        let result = resolve_time_range(false, false, false, Some("not-a-date"), None);
        assert!(result.is_err());
    }

    #[test]
    fn test_resolve_invalid_range() {
        let result = resolve_time_range(false, false, false, None, Some("bad-range"));
        assert!(result.is_err());
    }
}
