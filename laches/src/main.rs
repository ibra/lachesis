use auto_launch::AutoLaunch;
use clap::Parser;
use colored::Colorize;
use laches::{
    cli::{Cli, Commands, ListAction},
    process::{start_monitoring, stop_monitoring},
    process_list::ListMode,
    store::{
        get_stored_processes, load_or_create_store, reset_store, save_store, LachesStore, Process,
        STORE_NAME,
    },
    utils::{confirm, format_uptime},
};
use regex::Regex;
use std::{error::Error, path::Path, process::Command};

fn main() -> Result<(), Box<dyn Error>> {
    let store_path = match dirs::config_dir() {
        Some(dir) => dir.join("lachesis"),
        None => return Err("error: failed to get configuration directory".into()),
    };
    std::fs::create_dir_all(&store_path)?;

    let mut laches_store = load_or_create_store(&store_path)?;
    configure_daemon(&laches_store, &store_path);

    let cli = Cli::parse();

    match &cli.command {
        Commands::Autostart { toggle } => handle_autostart(toggle),
        Commands::Start => start_monitoring(&mut laches_store, &store_path),
        Commands::Stop => stop_monitoring(&mut laches_store),
        Commands::Mode { mode } => set_mode(mode, &mut laches_store),
        Commands::List { tag, today, date } => {
            list_processes(&laches_store, tag.as_deref(), *today, date.as_deref())
        }
        Commands::Tag {
            process,
            add,
            remove,
            list,
        } => handle_tag_command(
            &mut laches_store,
            process,
            add.as_deref(),
            remove.as_deref(),
            *list,
        ),
        Commands::Reset => confirm_reset_store(&store_path),
        Commands::Delete { all, duration } => {
            confirm_delete_data(&mut laches_store, *all, duration.as_deref())
        }
        Commands::Export { output, duration } => {
            export_data(&laches_store, output, duration.as_deref())
        }
        Commands::Whitelist { action } => handle_whitelist(&mut laches_store, action),
        Commands::Blacklist { action } => handle_blacklist(&mut laches_store, action),
    }?;

    save_store(&laches_store, &store_path)?;

    Ok(())
}

fn configure_daemon(laches_store: &LachesStore, store_path: &Path) {
    let mut monitor = Command::new("laches_mon");
    monitor
        .arg(&laches_store.update_interval.to_string())
        .arg(&store_path.join(STORE_NAME));
}

fn handle_autostart(toggle: &str) -> Result<(), Box<dyn Error>> {
    // Get the store path for laches_mon arguments
    let store_path = match dirs::config_dir() {
        Some(dir) => dir.join("lachesis"),
        None => return Err("error: failed to get configuration directory".into()),
    };

    let store_file = store_path.join(STORE_NAME);
    let laches_store = load_or_create_store(&store_path)?;

    // Find laches_mon executable
    let laches_mon_path = if cfg!(windows) {
        std::env::current_exe()?
            .parent()
            .ok_or("Failed to get parent directory")?
            .join("laches_mon.exe")
    } else {
        std::env::current_exe()?
            .parent()
            .ok_or("Failed to get parent directory")?
            .join("laches_mon")
    };

    // Verify laches_mon exists
    if !laches_mon_path.exists() {
        return Err(format!(
            "error: laches_mon executable not found at: {}",
            laches_mon_path.display()
        )
        .into());
    }

    // Build the command arguments for laches_mon
    let args = vec![
        laches_store.update_interval.to_string(),
        store_file.to_string_lossy().to_string(),
    ];

    // Create AutoLaunch configuration
    let auto = AutoLaunch::new(
        "laches_mon",
        laches_mon_path.to_str().ok_or("Invalid path")?,
        &args,
    );

    match toggle {
        "yes" => {
            if auto.is_enabled()? {
                println!("info: autostart is already enabled.");
            } else {
                auto.enable()?;
                println!("info: enabled laches_mon to run at startup.");
            }
        }
        "no" => {
            if !auto.is_enabled()? {
                println!("info: autostart is already disabled.");
            } else {
                auto.disable()?;
                println!("info: disabled laches_mon from running at startup.");
            }
        }
        _ => {
            return Err("error: invalid option for autostart. Use 'yes' or 'no'.".into());
        }
    }

    Ok(())
}

fn set_mode(mode: &str, laches_store: &mut LachesStore) -> Result<(), Box<dyn Error>> {
    match mode.parse::<ListMode>() {
        Ok(variant) => {
            laches_store.process_list_options.mode = variant;
            println!(
                "info: mode set to: {}",
                laches_store.process_list_options.mode.to_str()
            );
            Ok(())
        }
        Err(_) => Err(format!("error: no match found for mode: '{}'", mode).into()),
    }
}

fn list_processes(
    laches_store: &LachesStore,
    tag_filter: Option<&str>,
    today_only: bool,
    date_filter: Option<&str>,
) -> Result<(), Box<dyn Error>> {
    let all_windows = get_stored_processes(laches_store);

    // Determine display mode
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

    if let Some(tag) = tag_filter {
        println!(
            "{}",
            format!(
                "Tracked Window Usage - Tag: {} ({} Mode, {})",
                tag, mode_str, display_mode
            )
            .bold()
            .cyan()
        );
    } else {
        println!(
            "{}",
            format!("Tracked Window Usage ({} Mode, {})", mode_str, display_mode)
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

            // Apply tag filter
            let passes_tag = if let Some(tag) = tag_filter {
                window.tags.iter().any(|t| t == tag)
            } else {
                true
            };

            passes_mode && passes_tag
        })
        .collect();

    // Sort by appropriate usage
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

    // Find max usage for progress bar scaling
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

fn handle_tag_command(
    laches_store: &mut LachesStore,
    process_name: &str,
    add_tags: Option<&str>,
    remove_tags: Option<&str>,
    list_tags: bool,
) -> Result<(), Box<dyn Error>> {
    let process = laches_store
        .process_information
        .iter_mut()
        .find(|p| p.title == process_name);

    if process.is_none() {
        return Err(format!("error: process '{}' not found", process_name).into());
    }

    let process = process.unwrap();

    if list_tags {
        if process.tags.is_empty() {
            println!("Process '{}' has no tags", process_name);
        } else {
            println!("Tags for '{}': {}", process_name, process.tags.join(", "));
        }
        return Ok(());
    }

    if let Some(tags_str) = add_tags {
        let new_tags: Vec<String> = tags_str
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        for tag in new_tags {
            if !process.tags.contains(&tag) {
                process.tags.push(tag.clone());
                println!("Added tag '{}' to '{}'", tag, process_name);
            }
        }
    }

    if let Some(tags_str) = remove_tags {
        let remove_tags: Vec<String> = tags_str
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        for tag in remove_tags {
            if let Some(pos) = process.tags.iter().position(|t| t == &tag) {
                process.tags.remove(pos);
                println!("Removed tag '{}' from '{}'", tag, process_name);
            }
        }
    }

    Ok(())
}

fn confirm_reset_store(store_path: &Path) -> Result<(), Box<dyn Error>> {
    if confirm("are you sure you want to wipe the current store? [y/N]") {
        reset_store(store_path).expect("error: failed to reset store file");
    } else {
        println!("info: aborted reset operation");
    }

    Ok(())
}

fn confirm_delete_data(
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

fn export_data(
    laches_store: &LachesStore,
    output_path: &str,
    duration: Option<&str>,
) -> Result<(), Box<dyn Error>> {
    use std::fs::File;
    use std::io::Write;

    // Calculate cutoff date if duration is provided
    let cutoff_date = if let Some(duration_str) = duration {
        let days = parse_duration(duration_str)?;
        let cutoff = chrono::Local::now() - chrono::Duration::days(days);
        Some(cutoff.format("%Y-%m-%d").to_string())
    } else {
        None
    };

    // Create export data structure
    let mut export_processes: Vec<Process> = Vec::new();

    for process in &laches_store.process_information {
        let mut exported_process = process.clone();

        // Filter daily usage based on cutoff date if provided
        if let Some(ref cutoff) = cutoff_date {
            exported_process.daily_usage = process
                .daily_usage
                .iter()
                .filter(|(date, _)| date.as_str() >= cutoff.as_str())
                .map(|(k, v)| (k.clone(), *v))
                .collect();

            // Recalculate uptime based on filtered data
            exported_process.uptime = exported_process.daily_usage.values().sum();
        }

        // Only include processes with usage data
        if exported_process.uptime > 0 {
            export_processes.push(exported_process);
        }
    }

    // Sort by total usage
    export_processes.sort_by_key(|p| std::cmp::Reverse(p.get_total_usage()));

    // Create the JSON output
    let json_data = serde_json::to_string_pretty(&export_processes)?;

    // Write to file
    let mut file = File::create(output_path)?;
    file.write_all(json_data.as_bytes())?;

    // Print summary
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

    // Calculate and display total time
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

/// Check if a process name matches any pattern in the list (supports both exact matches and regex)
fn matches_any_pattern(process_name: &str, patterns: &[String]) -> bool {
    for pattern in patterns {
        // First try exact match
        if pattern == process_name {
            return true;
        }

        // Then try as regex if it looks like a regex pattern or doesn't match exactly
        if let Ok(regex) = Regex::new(pattern) {
            if regex.is_match(process_name) {
                return true;
            }
        }
    }
    false
}

fn handle_whitelist(
    laches_store: &mut LachesStore,
    action: &ListAction,
) -> Result<(), Box<dyn Error>> {
    match action {
        ListAction::Add { process, regex } => {
            add_to_list(laches_store, process, *regex, true)?;
        }
        ListAction::Remove { process } => {
            remove_from_list(laches_store, process, true)?;
        }
        ListAction::List => {
            list_patterns(laches_store, true)?;
        }
        ListAction::Clear => {
            clear_list(laches_store, true)?;
        }
    }
    Ok(())
}

fn handle_blacklist(
    laches_store: &mut LachesStore,
    action: &ListAction,
) -> Result<(), Box<dyn Error>> {
    match action {
        ListAction::Add { process, regex } => {
            add_to_list(laches_store, process, *regex, false)?;
        }
        ListAction::Remove { process } => {
            remove_from_list(laches_store, process, false)?;
        }
        ListAction::List => {
            list_patterns(laches_store, false)?;
        }
        ListAction::Clear => {
            clear_list(laches_store, false)?;
        }
    }
    Ok(())
}

fn add_to_list(
    laches_store: &mut LachesStore,
    pattern: &str,
    is_regex: bool,
    is_whitelist: bool,
) -> Result<(), Box<dyn Error>> {
    let list_name = if is_whitelist {
        "whitelist"
    } else {
        "blacklist"
    };

    // Validate regex if requested
    if is_regex {
        // Try to compile the regex
        let regex_result = Regex::new(pattern);
        if let Err(e) = regex_result {
            return Err(format!("error: invalid regex pattern: {}", e).into());
        }

        let regex = regex_result.unwrap();

        let existing_processes = &laches_store.process_information;
        let matched_processes: Vec<&String> = existing_processes
            .iter()
            .filter(|p| regex.is_match(&p.title))
            .map(|p| &p.title)
            .collect();

        println!(
            "{}",
            format!("Regex pattern '{}' will match:", pattern)
                .cyan()
                .bold()
        );
        if matched_processes.is_empty() {
            println!("  {}", "→ No currently tracked processes".bright_black());
            println!(
                "  {}",
                "  (pattern will apply to future processes)".bright_black()
            );
        } else {
            for proc in matched_processes.iter().take(10) {
                println!("  {} {}", "→".green(), proc.bright_white());
            }
            if matched_processes.len() > 10 {
                println!(
                    "  {}",
                    format!("  ... and {} more", matched_processes.len() - 10).bright_black()
                );
            }
        }
        println!();

        if !confirm(&format!(
            "add this regex pattern to the {}? [y/N]",
            list_name
        )) {
            println!("info: aborted operation");
            return Ok(());
        }
    }

    // Add to appropriate list
    let list = if is_whitelist {
        laches_store
            .process_list_options
            .whitelist
            .get_or_insert_with(Vec::new)
    } else {
        laches_store
            .process_list_options
            .blacklist
            .get_or_insert_with(Vec::new)
    };

    if list.contains(&pattern.to_string()) {
        println!(
            "{}",
            format!("info: '{}' is already in the {}", pattern, list_name).yellow()
        );
        return Ok(());
    }

    list.push(pattern.to_string());

    let pattern_type = if is_regex { "regex pattern" } else { "process" };
    println!(
        "{}",
        format!("✓ Added {} '{}' to {}", pattern_type, pattern, list_name).green()
    );

    Ok(())
}

fn remove_from_list(
    laches_store: &mut LachesStore,
    pattern: &str,
    is_whitelist: bool,
) -> Result<(), Box<dyn Error>> {
    let list_name = if is_whitelist {
        "whitelist"
    } else {
        "blacklist"
    };

    let list = if is_whitelist {
        &mut laches_store.process_list_options.whitelist
    } else {
        &mut laches_store.process_list_options.blacklist
    };

    if let Some(list_vec) = list {
        if let Some(pos) = list_vec.iter().position(|p| p == pattern) {
            list_vec.remove(pos);
            println!(
                "{}",
                format!("✓ Removed '{}' from {}", pattern, list_name).green()
            );

            // Clean up empty list
            if list_vec.is_empty() {
                *list = None;
            }
        } else {
            return Err(format!("error: '{}' not found in {}", pattern, list_name).into());
        }
    } else {
        return Err(format!("error: {} is empty", list_name).into());
    }

    Ok(())
}

fn list_patterns(laches_store: &LachesStore, is_whitelist: bool) -> Result<(), Box<dyn Error>> {
    let list_name = if is_whitelist {
        "Whitelist"
    } else {
        "Blacklist"
    };

    let list = if is_whitelist {
        &laches_store.process_list_options.whitelist
    } else {
        &laches_store.process_list_options.blacklist
    };

    println!("{}", format!("{} Patterns:", list_name).bold().cyan());
    println!();

    if let Some(patterns) = list {
        if patterns.is_empty() {
            println!(
                "  {}",
                format!("No patterns in {}", list_name.to_lowercase()).bright_black()
            );
        } else {
            for (i, pattern) in patterns.iter().enumerate() {
                // Try to detect if it looks like a regex
                let is_likely_regex = pattern.contains('[')
                    || pattern.contains(']')
                    || pattern.contains('(')
                    || pattern.contains(')')
                    || pattern.contains('*')
                    || pattern.contains('+')
                    || pattern.contains('?')
                    || pattern.contains('{')
                    || pattern.contains('}')
                    || pattern.contains('|')
                    || pattern.contains('^')
                    || pattern.contains('$')
                    || pattern.contains('\\');

                let pattern_type = if is_likely_regex {
                    format!(" {}", "[regex]".yellow())
                } else {
                    String::new()
                };

                println!("  {}. {}{}", i + 1, pattern.bright_white(), pattern_type);
            }
            println!();
            println!(
                "  {}",
                format!("Total: {} pattern(s)", patterns.len()).bright_black()
            );
        }
    } else {
        println!(
            "  {}",
            format!("No patterns in {}", list_name.to_lowercase()).bright_black()
        );
    }

    Ok(())
}

fn clear_list(laches_store: &mut LachesStore, is_whitelist: bool) -> Result<(), Box<dyn Error>> {
    let list_name = if is_whitelist {
        "whitelist"
    } else {
        "blacklist"
    };

    let list = if is_whitelist {
        &mut laches_store.process_list_options.whitelist
    } else {
        &mut laches_store.process_list_options.blacklist
    };

    if let Some(patterns) = list {
        let count = patterns.len();
        if count == 0 {
            println!(
                "{}",
                format!("info: {} is already empty", list_name).yellow()
            );
            return Ok(());
        }

        if confirm(&format!(
            "are you sure you want to clear all {} pattern(s) from the {}? [y/N]",
            count, list_name
        )) {
            *list = None;
            println!(
                "{}",
                format!("✓ Cleared {} pattern(s) from {}", count, list_name).green()
            );
        } else {
            println!("info: aborted operation");
        }
    } else {
        println!(
            "{}",
            format!("info: {} is already empty", list_name).yellow()
        );
    }

    Ok(())
}
