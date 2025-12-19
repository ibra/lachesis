use std::error::Error;

use crate::{cli::ListAction, store::LachesStore, utils::confirm};
use colored::Colorize;
use regex::Regex;

/// Check if a process name matches any pattern in the list (supports both exact matches and regex)
pub fn matches_any_pattern(process_name: &str, patterns: &[String]) -> bool {
    for pattern in patterns {
        if pattern == process_name {
            return true;
        }

        if let Ok(regex) = Regex::new(pattern) {
            if regex.is_match(process_name) {
                return true;
            }
        }
    }
    false
}

pub fn handle_whitelist(
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

pub fn handle_blacklist(
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

    if is_regex {
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
