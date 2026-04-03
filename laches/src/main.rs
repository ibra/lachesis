use clap::Parser;
use colored::Colorize;
use laches::{
    cli::{AutostartToggle, Cli, Commands, ConfigAction, DataAction, FilterMode},
    commands::autostart::handle_autostart,
    config::{load_or_create_config, save_config},
    db::{date_range_for_day, last_n_days_range, today_range, Database},
    process::{start_monitoring, stop_monitoring},
    store::get_machine_id,
    utils::format_uptime,
};
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let config_dir = match dirs::config_dir() {
        Some(dir) => dir.join("lachesis"),
        None => return Err("error: failed to get configuration directory".into()),
    };
    std::fs::create_dir_all(&config_dir)?;

    let mut config = load_or_create_config(&config_dir)?;
    let cli = Cli::parse();

    let machine_id = get_machine_id(&config_dir);
    let data_dir = laches::config::data_dir(&config_dir);
    std::fs::create_dir_all(&data_dir)?;

    let db_path = laches::config::machine_db_path(&config_dir, &machine_id);
    let db = Database::open(&db_path)?;

    match &cli.command {
        Commands::Start => start_monitoring(&config_dir),
        Commands::Stop => stop_monitoring(&config_dir),

        Commands::List {
            tag,
            today,
            date,
            all_machines: _,
        } => {
            let (start, end) = if let Some(d) = date {
                date_range_for_day(d).ok_or("error: invalid date format, use YYYY-MM-DD")?
            } else if *today {
                today_range()
            } else {
                last_n_days_range(365 * 10)
            };

            let label = if let Some(d) = date {
                format!("usage for {}", d)
            } else if *today {
                "today's usage".to_string()
            } else {
                "total usage".to_string()
            };

            let summaries = db.query_process_summaries(&start, &end, tag.as_deref())?;

            if summaries.is_empty() {
                println!("no tracked data for this period.");
                return Ok(());
            }

            let max_seconds = summaries.iter().map(|s| s.total_seconds).max().unwrap_or(1);

            if let Some(t) = tag {
                println!(
                    "{}",
                    format!("tracked usage - {} (tag: {})", label, t)
                        .bold()
                        .cyan()
                );
            } else {
                println!("{}", format!("tracked usage - {}", label).bold().cyan());
            }
            println!();

            for (i, s) in summaries.iter().enumerate() {
                let bar_width: usize = 20;
                let filled =
                    ((s.total_seconds as f64 / max_seconds as f64) * bar_width as f64) as usize;
                let empty = bar_width.saturating_sub(filled);
                let bar = format!(
                    "{}{}",
                    "#".repeat(filled).green(),
                    ".".repeat(empty).dimmed()
                );

                let tags = db.get_tags(&s.process_name).unwrap_or_default();
                let tag_str = if tags.is_empty() {
                    String::new()
                } else {
                    format!(" [{}]", tags.join(", "))
                };

                println!(
                    "  {:>2}. {:<24} {:>10}  {}{}",
                    i + 1,
                    s.process_name,
                    format_uptime(s.total_seconds as u64),
                    bar,
                    tag_str.dimmed(),
                );
            }

            let total: i64 = summaries.iter().map(|s| s.total_seconds).sum();
            println!();
            println!(
                "  {} processes, {} total",
                summaries.len(),
                format_uptime(total as u64)
            );

            Ok(())
        }

        Commands::Tag {
            process,
            add,
            remove,
            list,
        } => {
            if *list {
                let tags = db.get_tags(process)?;
                if tags.is_empty() {
                    println!("no tags for '{}'", process);
                } else {
                    println!("tags for '{}': {}", process, tags.join(", "));
                }
                return Ok(());
            }

            if let Some(add_tags) = add {
                for tag in add_tags
                    .split(',')
                    .map(|t| t.trim())
                    .filter(|t| !t.is_empty())
                {
                    db.add_tag(process, tag)?;
                    println!("added tag '{}' to '{}'", tag, process);
                }
            }

            if let Some(remove_tags) = remove {
                for tag in remove_tags
                    .split(',')
                    .map(|t| t.trim())
                    .filter(|t| !t.is_empty())
                {
                    if db.remove_tag(process, tag)? {
                        println!("removed tag '{}' from '{}'", tag, process);
                    } else {
                        println!("tag '{}' not found on '{}'", tag, process);
                    }
                }
            }

            Ok(())
        }

        Commands::Whitelist { action } => {
            handle_filter_list_action(&mut config, &config_dir, action, true)
        }
        Commands::Blacklist { action } => {
            handle_filter_list_action(&mut config, &config_dir, action, false)
        }

        Commands::Mode { mode } => {
            config.filtering.mode = match mode {
                FilterMode::Whitelist => "whitelist",
                FilterMode::Blacklist => "blacklist",
                FilterMode::Default => "default",
            }
            .to_string();
            save_config(&config, &config_dir)?;
            println!("filtering mode set to '{}'", config.filtering.mode);
            Ok(())
        }

        Commands::Autostart { toggle } => {
            let toggle_str = match toggle {
                AutostartToggle::On => "yes",
                AutostartToggle::Off => "no",
            };
            handle_autostart(toggle_str, &config_dir)
        }

        Commands::Config { action } => match action {
            Some(ConfigAction::StorePath { path: _ }) => {
                println!("info: store-path migration not yet implemented for sqlite");
                Ok(())
            }
            None => {
                println!("configuration:");
                println!("  config dir: {}", config_dir.display());
                println!("  machine id: {}", machine_id);
                println!("  check interval: {}s", config.daemon.check_interval);
                println!("  idle timeout: {}s", config.daemon.idle_timeout);
                println!("  filter mode: {}", config.filtering.mode);

                if !config.filtering.whitelist.is_empty() {
                    println!("  whitelist: {}", config.filtering.whitelist.join(", "));
                }
                if !config.filtering.blacklist.is_empty() {
                    println!("  blacklist: {}", config.filtering.blacklist.join(", "));
                }

                let data_dir = laches::config::data_dir(&config_dir);
                if data_dir.exists() {
                    let dbs: Vec<_> = std::fs::read_dir(&data_dir)?
                        .filter_map(|e| e.ok())
                        .filter(|e| e.path().extension().is_some_and(|ext| ext == "db"))
                        .collect();

                    if !dbs.is_empty() {
                        println!("\n  synced machines:");
                        for entry in dbs {
                            let name = entry.file_name();
                            let name = name.to_string_lossy().replace(".db", "");
                            let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
                            println!("    - {} ({:.1} KB)", name, size as f64 / 1024.0);
                        }
                    }
                }

                Ok(())
            }
        },

        Commands::Data { action } => match action {
            DataAction::Export {
                output,
                duration,
                all_machines: _,
            } => {
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

            DataAction::Delete { all, duration } => {
                if *all && duration.is_some() {
                    return Err("error: cannot specify both --all and --duration".into());
                }
                if !*all && duration.is_none() {
                    return Err("error: must specify either --all or --duration".into());
                }

                if *all {
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

            DataAction::Reset => {
                db.reset()?;
                println!("all sessions and tags cleared.");
                Ok(())
            }
        },
    }
}

fn handle_filter_list_action(
    config: &mut laches::config::Config,
    config_dir: &std::path::Path,
    action: &laches::cli::FilterListAction,
    is_whitelist: bool,
) -> Result<(), Box<dyn Error>> {
    let list = if is_whitelist {
        &mut config.filtering.whitelist
    } else {
        &mut config.filtering.blacklist
    };
    let list_name = if is_whitelist {
        "whitelist"
    } else {
        "blacklist"
    };

    match action {
        laches::cli::FilterListAction::Add { process, regex } => {
            if *regex {
                regex::Regex::new(process).map_err(|e| format!("error: invalid regex: {}", e))?;
            }
            if !list.contains(process) {
                list.push(process.clone());
                save_config(config, config_dir)?;
                println!("added '{}' to {}", process, list_name);
            } else {
                println!("'{}' is already in {}", process, list_name);
            }
        }
        laches::cli::FilterListAction::Remove { process } => {
            if let Some(pos) = list.iter().position(|p| p == process) {
                list.remove(pos);
                save_config(config, config_dir)?;
                println!("removed '{}' from {}", process, list_name);
            } else {
                println!("'{}' not found in {}", process, list_name);
            }
        }
        laches::cli::FilterListAction::List => {
            if list.is_empty() {
                println!("{} is empty", list_name);
            } else {
                println!("{}:", list_name);
                for pattern in list.iter() {
                    println!("  - {}", pattern);
                }
            }
        }
        laches::cli::FilterListAction::Clear => {
            list.clear();
            save_config(config, config_dir)?;
            println!("cleared {}", list_name);
        }
    }

    Ok(())
}

fn parse_duration_days(s: &str) -> Result<i64, Box<dyn Error>> {
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
