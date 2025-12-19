use std::{error::Error, fs::File, io::Write, path::Path};

use crate::{
    store::{reset_store, LachesStore, Process},
    utils::{confirm, format_uptime},
};
use colored::Colorize;

pub fn confirm_reset_store(store_path: &Path) -> Result<(), Box<dyn Error>> {
    if confirm("are you sure you want to wipe the current store? [y/N]") {
        reset_store(store_path).expect("error: failed to reset store file");
    } else {
        println!("info: aborted reset operation");
    }

    Ok(())
}

pub fn confirm_delete_store(
    laches_store: &mut LachesStore,
    delete_all: bool,
    duration: Option<&str>,
) -> Result<(), Box<dyn Error>> {
    if !delete_all && duration.is_none() {
        return Err("error: must specify either --all or --duration".into());
    }

    if delete_all && duration.is_some() {
        return Err("error: cannot specify both --all and --duration".into());
    }

    if delete_all {
        if confirm("are you sure you want to delete all recorded time? [y/N]") {
            let total_processes = laches_store.process_information.len();
            for process in &mut laches_store.process_information {
                process.daily_usage.clear();
                process.uptime = 0;
            }
            println!(
                "info: deleted all recorded time from {} process(es)",
                total_processes
            );
        } else {
            println!("info: aborted delete operation");
        }
    } else if let Some(duration_str) = duration {
        let days = parse_duration(duration_str)?;
        let cutoff_date = chrono::Local::now() - chrono::Duration::days(days);
        let cutoff_str = cutoff_date.format("%Y-%m-%d").to_string();

        if confirm(&format!(
            "are you sure you want to delete data older than {} days (before {})? [y/N]",
            days, cutoff_str
        )) {
            let mut total_deleted = 0;
            for process in &mut laches_store.process_information {
                let dates_to_remove: Vec<String> = process
                    .daily_usage
                    .keys()
                    .filter(|date| *date < &cutoff_str)
                    .cloned()
                    .collect();

                for date in dates_to_remove {
                    if let Some(usage) = process.daily_usage.remove(&date) {
                        process.uptime = process.uptime.saturating_sub(usage);
                        total_deleted += 1;
                    }
                }
            }
            println!(
                "info: deleted {} daily record(s) older than {} days",
                total_deleted, days
            );
        } else {
            println!("info: aborted delete operation");
        }
    }

    Ok(())
}

pub fn export_store(
    laches_store: &LachesStore,
    output_path: &str,
    duration: Option<&str>,
) -> Result<(), Box<dyn Error>> {
    let cutoff_date = if let Some(duration_str) = duration {
        let days = parse_duration(duration_str)?;
        let cutoff = chrono::Local::now() - chrono::Duration::days(days);
        Some(cutoff.format("%Y-%m-%d").to_string())
    } else {
        None
    };

    let mut export_processes: Vec<Process> = Vec::new();

    for process in &laches_store.process_information {
        let mut exported_process = process.clone();

        if let Some(ref cutoff) = cutoff_date {
            exported_process.daily_usage = process
                .daily_usage
                .iter()
                .filter(|(date, _)| date.as_str() >= cutoff.as_str())
                .map(|(k, v)| (k.clone(), *v))
                .collect();

            exported_process.uptime = exported_process.daily_usage.values().sum();
        }

        if exported_process.uptime > 0 {
            export_processes.push(exported_process);
        }
    }

    export_processes.sort_by_key(|p| std::cmp::Reverse(p.get_total_usage()));
    let json_data = serde_json::to_string_pretty(&export_processes)?;

    let mut file = File::create(output_path)?;
    file.write_all(json_data.as_bytes())?;

    let duration_text = if let Some(duration_str) = duration {
        format!(" (past {})", duration_str)
    } else {
        " (all time)".to_string()
    };

    println!(
        "{}",
        format!(
            "✓ Exported {} process(es){} to '{}'",
            export_processes.len(),
            duration_text,
            output_path
        )
        .green()
    );

    let total_time: u64 = export_processes.iter().map(|p| p.get_total_usage()).sum();
    let formatted_total = format_uptime(total_time);
    println!(
        "{}",
        format!("  Total tracked time: {}", formatted_total).bright_black()
    );

    Ok(())
}

fn parse_duration(duration_str: &str) -> Result<i64, Box<dyn Error>> {
    if !duration_str.ends_with('d') {
        return Err("error: duration must be in format like '7d', '30d', etc.".into());
    }

    let days_str = &duration_str[..duration_str.len() - 1];
    let days = days_str
        .parse::<i64>()
        .map_err(|_| "error: invalid duration value")?;

    if days <= 0 {
        return Err("error: duration must be a positive number".into());
    }

    Ok(days)
}
