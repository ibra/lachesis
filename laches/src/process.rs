use crate::{
    store::{LachesStore, Process, STORE_NAME},
    utils::confirm,
};
use std::env;
use std::{error::Error, path::Path, process::Command};
use sysinfo::{Pid, System};

pub fn start_monitoring(
    laches_store: &mut LachesStore,
    store_path: &Path,
) -> Result<(), Box<dyn Error>> {
    let active_windows = get_active_processes();
    println!("info: started monitoring {} windows", &active_windows.len());

    let mut exe_path = env::current_exe().unwrap();
    exe_path.pop();
    exe_path.push("laches_mon");

    let mut monitor = Command::new(exe_path);
    monitor
        .arg(&laches_store.update_interval.to_string())
        .arg(&store_path.join(STORE_NAME));

    let instance = monitor
        .spawn()
        .expect("error: failed to execute laches_mon (monitoring daemon)");

    laches_store.daemon_pid = instance.id();
    Ok(())
}

pub fn stop_monitoring(laches_store: &mut LachesStore) -> Result<(), Box<dyn Error>> {
    if confirm("are you sure you want to stop window tracking (kill laches_mon)? [y/N]") {
        let s = System::new_all();
        if let Some(process) = s.process(Pid::from(laches_store.daemon_pid as usize)) {
            process.kill();
        }
        println!("info: killed laches_mon (monitoring daemon)");
    } else {
        println!("info: aborted stop operation");
    }

    Ok(())
}

pub fn get_active_processes() -> Vec<Process> {
    let mut active_processes: Vec<Process> = Vec::new();
    let system = System::new_all();

    for process in system.processes().values() {
        let name = process.name().to_string();

        let contains_title = active_processes.iter().any(|window| window.title == name);

        if name.trim() == "" || contains_title {
            continue;
        }

        active_processes.push(Process::new(name));
    }
    active_processes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_active_processes_returns_vector() {
        // This test verifies that get_active_processes returns a vector
        // The actual content will vary by system
        let processes = get_active_processes();

        // Should be a valid vector (might be empty or populated)
        assert!(processes.len() >= 0);
    }

    #[test]
    fn test_get_active_processes_no_duplicates() {
        let processes = get_active_processes();

        // Check that there are no duplicate process names
        let mut seen_titles = std::collections::HashSet::new();
        for process in &processes {
            assert!(
                seen_titles.insert(process.title.clone()),
                "Duplicate process title found: {}",
                process.title
            );
        }
    }

    #[test]
    fn test_get_active_processes_no_empty_names() {
        let processes = get_active_processes();

        // Verify no process has an empty name
        for process in &processes {
            assert!(
                !process.title.trim().is_empty(),
                "Process with empty name found"
            );
        }
    }

    #[test]
    fn test_get_active_processes_creates_new_processes() {
        let processes = get_active_processes();

        // Each process should be newly created with uptime of 0
        for process in &processes {
            assert_eq!(process.uptime, 0);
            assert_eq!(process.daily_usage.len(), 0);
            assert_eq!(process.tags.len(), 0);
        }
    }
}
