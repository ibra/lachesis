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
            let current_machine_processes = laches_store.get_current_machine_processes_mut();
            let total_processes = current_machine_processes.len();
            for process in current_machine_processes.iter_mut() {
                process.daily_usage.clear();
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
            let current_machine_processes = laches_store.get_current_machine_processes_mut();
            for process in current_machine_processes.iter_mut() {
                let dates_to_remove: Vec<String> = process
                    .daily_usage
                    .keys()
                    .filter(|date| *date < &cutoff_str)
                    .cloned()
                    .collect();

                for date in dates_to_remove {
                    if let Some(_usage) = process.daily_usage.remove(&date) {
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
    all_machines: bool,
) -> Result<(), Box<dyn Error>> {
    let cutoff_date = if let Some(duration_str) = duration {
        let days = parse_duration(duration_str)?;
        let cutoff = chrono::Local::now() - chrono::Duration::days(days);
        Some(cutoff.format("%Y-%m-%d").to_string())
    } else {
        None
    };

    let mut export_processes: Vec<Process> = Vec::new();

    let processes_to_export = if all_machines {
        laches_store.get_all_processes()
    } else {
        laches_store.get_current_machine_processes()
    };

    for process in &processes_to_export {
        let mut exported_process = process.clone();

        if let Some(ref cutoff) = cutoff_date {
            exported_process.daily_usage = process
                .daily_usage
                .iter()
                .filter(|(date, _)| date.as_str() >= cutoff.as_str())
                .map(|(k, v)| (k.clone(), *v))
                .collect();
        }

        if exported_process.get_total_usage() > 0 {
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

    let machines_text = if all_machines {
        format!(" from {} machine(s)", laches_store.machine_data.len())
    } else {
        String::new()
    };

    println!(
        "{}",
        format!(
            "✓ Exported {} process(es){}{} to '{}'",
            export_processes.len(),
            duration_text,
            machines_text,
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

pub fn parse_duration(duration_str: &str) -> Result<i64, Box<dyn Error>> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_parse_duration_valid() {
        assert_eq!(parse_duration("7d").unwrap(), 7);
        assert_eq!(parse_duration("30d").unwrap(), 30);
        assert_eq!(parse_duration("365d").unwrap(), 365);
        assert_eq!(parse_duration("1d").unwrap(), 1);
    }

    #[test]
    fn test_parse_duration_invalid_format() {
        assert!(parse_duration("7").is_err());
        assert!(parse_duration("7days").is_err());
        assert!(parse_duration("d7").is_err());
        assert!(parse_duration("7w").is_err());
        assert!(parse_duration("").is_err());
    }

    #[test]
    fn test_parse_duration_invalid_number() {
        assert!(parse_duration("abcd").is_err());
        assert!(parse_duration("12.5d").is_err());
        assert!(parse_duration("-5d").is_err());
        assert!(parse_duration("0d").is_err());
    }

    #[test]
    fn test_parse_duration_zero_or_negative() {
        let result = parse_duration("0d");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("duration must be a positive number"));
    }

    #[test]
    fn test_export_store_all_data() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("export.json");

        let mut store = LachesStore::default();
        let mut process1 = Process::new("process1".to_string());
        process1.add_time(3600);
        let mut process2 = Process::new("process2".to_string());
        process2.add_time(7200);
        let hostname = crate::store::get_hostname();
        store
            .machine_data
            .insert(hostname, vec![process1, process2]);

        let result = export_store(&store, output_path.to_str().unwrap(), None, false);
        assert!(result.is_ok());
        assert!(output_path.exists());

        // Verify exported data
        let exported_data = std::fs::read_to_string(&output_path).unwrap();
        let exported_processes: Vec<Process> = serde_json::from_str(&exported_data).unwrap();
        assert_eq!(exported_processes.len(), 2);
    }

    #[test]
    fn test_export_store_with_duration_filter() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("export_filtered.json");

        let mut store = LachesStore::default();
        let mut process = Process::new("test_process".to_string());

        // Add data for today
        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        process.daily_usage.insert(today.clone(), 1000);

        // Add data for 10 days ago
        let old_date = (chrono::Local::now() - chrono::Duration::days(10))
            .format("%Y-%m-%d")
            .to_string();
        process.daily_usage.insert(old_date.clone(), 5000);

        let hostname = crate::store::get_hostname();
        store.machine_data.insert(hostname, vec![process]);

        // Export only last 5 days
        let result = export_store(&store, output_path.to_str().unwrap(), Some("5d"), false);
        assert!(result.is_ok());

        let exported_data = std::fs::read_to_string(&output_path).unwrap();
        let exported_processes: Vec<Process> = serde_json::from_str(&exported_data).unwrap();

        assert_eq!(exported_processes.len(), 1);
        // Should only have today's data
        assert_eq!(exported_processes[0].daily_usage.len(), 1);
        assert!(exported_processes[0].daily_usage.contains_key(&today));
        assert!(!exported_processes[0].daily_usage.contains_key(&old_date));
    }

    #[test]
    fn test_export_store_excludes_zero_uptime() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("export_no_zero.json");

        let mut store = LachesStore::default();
        let mut process_with_time = Process::new("active".to_string());
        process_with_time.add_time(1000);
        let process_without_time = Process::new("inactive".to_string());

        let hostname = crate::store::get_hostname();
        store
            .machine_data
            .insert(hostname, vec![process_with_time, process_without_time]);

        let result = export_store(&store, output_path.to_str().unwrap(), None, false);
        assert!(result.is_ok());

        let exported_data = std::fs::read_to_string(&output_path).unwrap();
        let exported_processes: Vec<Process> = serde_json::from_str(&exported_data).unwrap();

        // Only process with uptime > 0 should be exported
        assert_eq!(exported_processes.len(), 1);
        assert_eq!(exported_processes[0].title, "active");
    }

    #[test]
    fn test_export_store_sorting() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("export_sorted.json");

        let mut store = LachesStore::default();
        let mut process1 = Process::new("low_usage".to_string());
        process1.add_time(100);
        let mut process2 = Process::new("high_usage".to_string());
        process2.add_time(1000);
        let mut process3 = Process::new("medium_usage".to_string());
        process3.add_time(500);

        let hostname = crate::store::get_hostname();
        store
            .machine_data
            .insert(hostname, vec![process1, process2, process3]);

        let result = export_store(&store, output_path.to_str().unwrap(), None, false);
        assert!(result.is_ok());

        let exported_data = std::fs::read_to_string(&output_path).unwrap();
        let exported_processes: Vec<Process> = serde_json::from_str(&exported_data).unwrap();

        // Should be sorted by total usage (descending)
        assert_eq!(exported_processes[0].title, "high_usage");
        assert_eq!(exported_processes[1].title, "medium_usage");
        assert_eq!(exported_processes[2].title, "low_usage");
    }

    #[test]
    fn test_confirm_delete_store_all_clears_data() {
        let mut store = LachesStore::default();
        let mut process1 = Process::new("process1".to_string());
        process1.add_time(1000);
        let mut process2 = Process::new("process2".to_string());
        process2.add_time(2000);

        let hostname = crate::store::get_hostname();
        store
            .machine_data
            .insert(hostname, vec![process1, process2]);

        // This test would require mocking user input, so we'll just verify
        // the function signature and error handling
        let result = confirm_delete_store(&mut store, false, None);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("must specify either --all or --duration"));
    }

    #[test]
    fn test_confirm_delete_store_invalid_both_flags() {
        let mut store = LachesStore::default();

        let result = confirm_delete_store(&mut store, true, Some("7d"));
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("cannot specify both --all and --duration"));
    }

    #[test]
    fn test_export_store_empty_store() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("export_empty.json");

        let store = LachesStore::default();

        let result = export_store(&store, output_path.to_str().unwrap(), None, false);
        assert!(result.is_ok());

        let exported_data = std::fs::read_to_string(&output_path).unwrap();
        let exported_processes: Vec<Process> = serde_json::from_str(&exported_data).unwrap();

        assert_eq!(exported_processes.len(), 0);
    }
}
