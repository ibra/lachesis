use clap::Parser;
use laches::{
    cli::{Cli, Commands, ConfigAction, DataAction},
    commands::{
        autostart::handle_autostart,
        data,
        list::{print_process_summaries, print_sessions, resolve_time_range},
        summary::print_summary,
    },
    config::{get_machine_id, load_or_create_config, save_config, FilterPattern},
    db::Database,
    process::{start_monitoring, stop_monitoring},
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
        Commands::Start => Ok(start_monitoring(&config_dir)?),
        Commands::Stop => Ok(stop_monitoring(&config_dir)?),

        Commands::List {
            tag,
            today,
            week,
            month,
            date,
            range,
            sessions,
            verbose,
            all_machines,
        } => {
            if *all_machines {
                eprintln!(
                    "warning: --all-machines is not yet implemented, showing local machine only"
                );
            }
            let (start, end, label) =
                resolve_time_range(*today, *week, *month, date.as_deref(), range.as_deref())?;

            if *sessions {
                print_sessions(&db, &start, &end, &label)?;
            } else {
                print_process_summaries(&db, &start, &end, &label, tag.as_deref(), *verbose)?;
            }

            Ok(())
        }

        Commands::Summary => print_summary(&db),

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
            config.filtering.mode = mode.clone().into();
            save_config(&config, &config_dir)?;
            println!("filtering mode set to '{}'", config.filtering.mode);
            Ok(())
        }

        Commands::Autostart { toggle } => handle_autostart(toggle, &config_dir),

        Commands::Config { action } => match action {
            Some(ConfigAction::StorePath { path: _ }) => {
                eprintln!("warning: store-path is not yet implemented");
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
                    let patterns: Vec<String> = config
                        .filtering
                        .whitelist
                        .iter()
                        .map(|p| p.to_string())
                        .collect();
                    println!("  whitelist: {}", patterns.join(", "));
                }
                if !config.filtering.blacklist.is_empty() {
                    let patterns: Vec<String> = config
                        .filtering
                        .blacklist
                        .iter()
                        .map(|p| p.to_string())
                        .collect();
                    println!("  blacklist: {}", patterns.join(", "));
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
                all_machines,
            } => {
                if *all_machines {
                    eprintln!("warning: --all-machines is not yet implemented, exporting local machine only");
                }
                data::export_sessions(&db, output, duration.as_deref())
            }

            DataAction::Delete { all, duration } => {
                data::delete_sessions(&db, *all, duration.as_deref())
            }

            DataAction::Reset => data::reset_data(&db),
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
            let pattern = if *regex {
                FilterPattern::regex(process)
            } else {
                FilterPattern::exact(process)
            };
            if list.iter().any(|p| p.pattern == pattern.pattern) {
                println!("'{}' is already in {}", process, list_name);
            } else {
                list.push(pattern);
                save_config(config, config_dir)?;
                println!("added '{}' to {}", process, list_name);
            }
        }
        laches::cli::FilterListAction::Remove { process } => {
            if let Some(pos) = list.iter().position(|p| p.pattern == *process) {
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
