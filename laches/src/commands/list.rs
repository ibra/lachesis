use std::{error::Error, path::Path};

use crate::{
    commands::filtering::matches_any_pattern,
    process_list::ListMode,
    store::{LachesStore, Process},
    utils::format_uptime,
};
use colored::Colorize;
use tabled::{
    builder::Builder,
    settings::{object::Segment, style::Style, Alignment, Modify},
};

#[derive(Debug)]
struct ProcessStats {
    title: String,
    today_usage: u64,
    total_usage: u64,
    active_days: usize,
    avg_per_day: u64,
    tags: Vec<String>,
}

impl ProcessStats {
    fn from_process(process: &Process, _date_filter: Option<&str>, _today_only: bool) -> Self {
        let today_usage = process.get_today_usage();
        let total_usage = process.get_total_usage();
        let active_days = process.daily_usage.len();
        let avg_per_day = if active_days > 0 {
            total_usage / active_days as u64
        } else {
            0
        };

        Self {
            title: process.title.clone(),
            today_usage,
            total_usage,
            active_days,
            avg_per_day,
            tags: process.tags.clone(),
        }
    }

    fn get_display_usage(&self, date_filter: Option<(&str, u64)>, today_only: bool) -> u64 {
        if let Some((_, usage)) = date_filter {
            usage
        } else if today_only {
            self.today_usage
        } else {
            self.total_usage
        }
    }
}

fn create_progress_bar(value: u64, max_value: u64, width: usize) -> String {
    if max_value == 0 {
        return "░".repeat(width);
    }

    let percentage = (value as f64 / max_value as f64) * 100.0;
    let filled = ((percentage / 100.0) * width as f64) as usize;
    let empty = width.saturating_sub(filled);

    format!("{}{}", "█".repeat(filled), "░".repeat(empty))
}

pub fn list_processes(
    laches_store: &LachesStore,
    store_path: &Path,
    tag_filter: Option<&str>,
    today_only: bool,
    date_filter: Option<&str>,
    all_machines: bool,
) -> Result<(), Box<dyn Error>> {
    let all_windows = if all_machines {
        laches_store.get_all_processes()
    } else {
        laches_store.get_machine_processes(store_path)
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

    // Header
    if let Some(tag) = tag_filter {
        println!(
            "{}",
            format!(
                "📊 Tracked Window Usage - Tag: {} ({} Mode, {}{})",
                tag, mode_str, display_mode, machines_str
            )
            .bold()
            .cyan()
        );
    } else {
        println!(
            "{}",
            format!(
                "📊 Tracked Window Usage ({} Mode, {}{})",
                mode_str, display_mode, machines_str
            )
            .bold()
            .cyan()
        );
    }
    println!();

    let filtered_windows: Vec<Process> = all_windows
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

    let mut stats: Vec<(ProcessStats, u64)> = filtered_windows
        .iter()
        .filter_map(|w| {
            let display_usage = if let Some(date) = date_filter {
                *w.daily_usage.get(date).unwrap_or(&0)
            } else if today_only {
                w.get_today_usage()
            } else {
                w.get_total_usage()
            };

            if display_usage > 0 {
                Some((
                    ProcessStats::from_process(w, date_filter, today_only),
                    display_usage,
                ))
            } else {
                None
            }
        })
        .collect();

    if stats.is_empty() {
        println!("{}", "⚠️  No monitored windows found".yellow());
        println!();

        if laches_store.process_list_options.mode == ListMode::Whitelist {
            println!(
                "{}",
                "Tip: You're in Whitelist mode. Make sure you've added patterns to your whitelist."
                    .bright_black()
            );
            println!(
                "{}",
                "     Use: laches mode whitelist add <pattern>".bright_black()
            );
        } else if laches_store.process_list_options.mode == ListMode::Default {
            println!(
                "{}",
                "Tip: In Default mode, all windows are tracked. Make sure laches_mon is running."
                    .bright_black()
            );
            println!(
                "{}",
                "     The monitor daemon should be adding window data automatically."
                    .bright_black()
            );
        }

        return Ok(());
    }

    stats.sort_by_key(|(_, usage)| std::cmp::Reverse(*usage));

    let max_usage = stats.iter().map(|(_, u)| *u).max().unwrap_or(1);
    let total_usage: u64 = stats.iter().map(|(_, u)| *u).sum();
    let total_processes = stats.len();

    let mut builder = Builder::default();

    builder.push_record(vec![
        "#",
        "Window Title",
        "Usage",
        "Progress",
        "%",
        "Active Days",
        "Avg/Day",
        "Tags",
    ]);

    for (idx, (stat, display_usage)) in stats.iter().enumerate() {
        let rank = (idx + 1).to_string();

        let title = if stat.title.len() > 40 {
            format!("{}...", &stat.title[..37])
        } else {
            stat.title.clone()
        };

        let usage_str = format_uptime(
            stat.get_display_usage(date_filter.map(|d| (d, *display_usage)), today_only),
        );

        let progress_bar = create_progress_bar(*display_usage, max_usage, 25);

        let percentage = (*display_usage as f64 / max_usage as f64) * 100.0;
        let percentage_str = format!("{:.1}", percentage);

        let active_days = if date_filter.is_some() || today_only {
            "-".to_string()
        } else {
            stat.active_days.to_string()
        };

        let avg_per_day = if date_filter.is_some() || today_only {
            "-".to_string()
        } else {
            format_uptime(stat.avg_per_day)
        };

        let tags_str = if stat.tags.is_empty() {
            "-".to_string()
        } else {
            stat.tags.join(", ")
        };

        builder.push_record(vec![
            &rank,
            &title,
            &usage_str,
            &progress_bar,
            &percentage_str,
            &active_days,
            &avg_per_day,
            &tags_str,
        ]);
    }

    let mut table = builder.build();

    // Apply styling
    table
        .with(Style::rounded())
        .with(Modify::new(Segment::all()).with(Alignment::left()))
        .with(Modify::new(tabled::settings::object::Columns::single(0)).with(Alignment::center())) // # column
        .with(Modify::new(tabled::settings::object::Columns::single(2)).with(Alignment::right())) // Usage column
        .with(Modify::new(tabled::settings::object::Columns::single(4)).with(Alignment::right())) // % column
        .with(Modify::new(tabled::settings::object::Columns::single(5)).with(Alignment::center())) // Active Days column
        .with(Modify::new(tabled::settings::object::Columns::single(6)).with(Alignment::right())); // Avg/Day column

    println!("{}", table);
    println!();

    // Summary statistics
    println!("{}", "📈 Summary Statistics".bold().cyan());
    println!("{}", "─".repeat(60).bright_black());

    let avg_usage_per_process = if total_processes > 0 {
        total_usage / total_processes as u64
    } else {
        0
    };

    println!(
        "  {} {}",
        "Total Processes:".bright_white(),
        total_processes.to_string().yellow()
    );

    println!(
        "  {} {}",
        "Total Time Tracked:".bright_white(),
        format_uptime(total_usage).yellow()
    );

    println!(
        "  {} {}",
        "Average per Process:".bright_white(),
        format_uptime(avg_usage_per_process).yellow()
    );

    if !date_filter.is_some() && !today_only {
        // Calculate total active days across all processes
        let total_active_days: usize = stats.iter().map(|(s, _)| s.active_days).sum();
        let avg_active_days = if total_processes > 0 {
            total_active_days as f64 / total_processes as f64
        } else {
            0.0
        };

        println!(
            "  {} {:.1}",
            "Avg Active Days:".bright_white(),
            avg_active_days.to_string().yellow()
        );

        if let Some((top_stat, top_usage)) = stats.first() {
            let top_percentage = (*top_usage as f64 / total_usage as f64) * 100.0;
            println!(
                "  {} {} ({:.1}%)",
                "Most Used:".bright_white(),
                top_stat.title.green(),
                top_percentage
            );
        }
    }

    Ok(())
}
