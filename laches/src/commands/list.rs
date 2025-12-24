use std::error::Error;

use crate::{
    commands::filtering::matches_any_pattern,
    process_list::ListMode,
    store::{get_stored_processes, LachesStore, Process},
    utils::format_uptime,
};
use colored::Colorize;

pub fn list_processes(
    laches_store: &LachesStore,
    tag_filter: Option<&str>,
    today_only: bool,
    date_filter: Option<&str>,
    all_machines: bool,
) -> Result<(), Box<dyn Error>> {
    let all_windows = if all_machines {
        laches_store.get_all_processes()
    } else {
        get_stored_processes(laches_store)
    };

    let display_mode = if let Some(date) = date_filter {
        format!("Usage for {}", date)
    } else if today_only {
        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        format!("Today's Usage ({})", today)
    } else {
        "Total Usage".to_string()
    };

    let mode_str = match laches_store.process_list_options.mode {
        ListMode::Whitelist => "Whitelist",
        ListMode::Blacklist => "Blacklist",
        ListMode::Default => "Default",
    };

    let machines_str = if all_machines {
        format!(
            " - All Machines ({} total)",
            laches_store.machine_data.len()
        )
    } else {
        String::new()
    };

    if let Some(tag) = tag_filter {
        println!(
            "{}",
            format!(
                "Tracked Window Usage - Tag: {} ({} Mode, {}{})",
                tag, mode_str, display_mode, machines_str
            )
            .bold()
            .cyan()
        );
    } else {
        println!(
            "{}",
            format!(
                "Tracked Window Usage ({} Mode, {}{})",
                mode_str, display_mode, machines_str
            )
            .bold()
            .cyan()
        );
    }
    println!();

    // Filter and sort windows
    let mut filtered_windows: Vec<Process> = all_windows
        .into_iter()
        .filter(|window| {
            // Apply whitelist/blacklist with regex support
            let passes_mode = match laches_store.process_list_options.mode {
                ListMode::Whitelist => {
                    let whitelist = laches_store
                        .process_list_options
                        .whitelist
                        .as_deref()
                        .unwrap_or(&[]);
                    matches_any_pattern(&window.title, whitelist)
                }
                ListMode::Blacklist => {
                    let blacklist = laches_store
                        .process_list_options
                        .blacklist
                        .as_deref()
                        .unwrap_or(&[]);
                    !matches_any_pattern(&window.title, blacklist)
                }
                ListMode::Default => true,
            };

            let passes_tag = if let Some(tag) = tag_filter {
                window.tags.iter().any(|t| t == tag)
            } else {
                true
            };

            passes_mode && passes_tag
        })
        .collect();

    if let Some(date) = date_filter {
        filtered_windows.sort_by_key(|w| std::cmp::Reverse(*w.daily_usage.get(date).unwrap_or(&0)));
    } else if today_only {
        filtered_windows.sort_by_key(|w| std::cmp::Reverse(w.get_today_usage()));
    } else {
        filtered_windows.sort_by_key(|w| std::cmp::Reverse(w.get_total_usage()));
    }

    if filtered_windows.is_empty() {
        println!("{}", "warning: no monitored windows".yellow());
        return Ok(());
    }

    let max_usage = if let Some(date) = date_filter {
        filtered_windows
            .iter()
            .map(|w| *w.daily_usage.get(date).unwrap_or(&0))
            .max()
            .unwrap_or(1)
    } else if today_only {
        filtered_windows
            .iter()
            .map(|w| w.get_today_usage())
            .max()
            .unwrap_or(1)
    } else {
        filtered_windows
            .iter()
            .map(|w| w.get_total_usage())
            .max()
            .unwrap_or(1)
    };

    // Display processes with progress bars
    for window in &filtered_windows {
        let usage = if let Some(date) = date_filter {
            *window.daily_usage.get(date).unwrap_or(&0)
        } else if today_only {
            window.get_today_usage()
        } else {
            window.get_total_usage()
        };

        if usage == 0 {
            continue;
        }

        let formatted_time = format_uptime(usage);
        let percentage = (usage as f64 / max_usage as f64) * 100.0;
        let bar_length = 40;
        let filled = ((percentage / 100.0) * bar_length as f64) as usize;
        let empty = bar_length - filled;

        let bar = format!(
            "{}{}",
            "█".repeat(filled).green(),
            "░".repeat(empty).bright_black()
        );

        // Show tags if present
        let tag_display = if !window.tags.is_empty() {
            format!(" {}", format!("[{}]", window.tags.join(", ")).bright_blue())
        } else {
            String::new()
        };

        println!(
            "{:40} {} {:>12} {:>6.1}%{}",
            window.title.bright_white(),
            bar,
            formatted_time.yellow(),
            percentage,
            tag_display
        );
    }

    println!();
    println!(
        "{}",
        format!("Total processes: {}", filtered_windows.len()).bright_black()
    );

    Ok(())
}
