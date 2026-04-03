use crate::db::{date_range_for_day, last_n_days_range, today_range, Database};
use crate::utils::format_uptime;
use colored::Colorize;
use std::error::Error;

/// Print a quick daily overview with comparisons to yesterday and weekly average.
pub fn print_summary(db: &Database) -> Result<(), Box<dyn Error>> {
    let (today_start, today_end) = today_range();
    let today_active = db.query_total_active_seconds(&today_start, &today_end)?;
    let today_idle = db.query_total_idle_seconds(&today_start, &today_end)?;
    let summaries = db.query_process_summaries(&today_start, &today_end, None)?;

    // yesterday
    let yesterday = (chrono::Local::now() - chrono::Duration::days(1))
        .format("%Y-%m-%d")
        .to_string();
    let (yday_start, yday_end) = date_range_for_day(&yesterday).unwrap_or_else(today_range);
    let yesterday_active = db.query_total_active_seconds(&yday_start, &yday_end)?;

    // weekly average
    let (week_start, week_end) = last_n_days_range(7);
    let week_total = db.query_total_active_seconds(&week_start, &week_end)?;
    let week_avg = week_total / 7;

    // header
    let idle_str = if today_idle > 0 {
        format!(" ({}idle)", format_uptime(today_idle as u64) + " ")
    } else {
        String::new()
    };
    println!(
        "{}",
        format!("today: {}{}", format_uptime(today_active as u64), idle_str,)
            .bold()
            .cyan()
    );
    println!();

    // top 5 processes
    let max_seconds = summaries
        .iter()
        .take(5)
        .map(|s| s.total_seconds)
        .max()
        .unwrap_or(1);

    for (i, s) in summaries.iter().take(5).enumerate() {
        let bar_width: usize = 15;
        let filled = ((s.total_seconds as f64 / max_seconds as f64) * bar_width as f64) as usize;
        let empty = bar_width.saturating_sub(filled);
        let bar = format!(
            "{}{}",
            "#".repeat(filled).green(),
            ".".repeat(empty).dimmed()
        );

        println!(
            "  {:>2}. {:<22} {:>10}  {}",
            i + 1,
            s.process_name,
            format_uptime(s.total_seconds as u64),
            bar,
        );
    }

    if summaries.len() > 5 {
        let rest: i64 = summaries[5..].iter().map(|s| s.total_seconds).sum();
        println!(
            "      +{} others {:>14}",
            summaries.len() - 5,
            format_uptime(rest as u64),
        );
    }

    if summaries.is_empty() {
        println!("  no tracked data for today.");
    }

    // comparisons
    println!();
    let vs_yesterday = today_active - yesterday_active;
    let vs_week = today_active - week_avg;

    let fmt_delta = |d: i64| -> String {
        use std::cmp::Ordering;
        match d.cmp(&0) {
            Ordering::Greater => format!("+{}", format_uptime(d as u64)),
            Ordering::Less => format!("-{}", format_uptime((-d) as u64)),
            Ordering::Equal => "same".to_string(),
        }
    };

    println!(
        "  vs yesterday: {}  |  vs weekly avg: {}",
        fmt_delta(vs_yesterday),
        fmt_delta(vs_week),
    );

    Ok(())
}
