use crate::config::{clear_daemon_pid, read_daemon_pid, write_daemon_pid};
use crate::store::{normalize_process_name, Process};
use std::env;
use std::process::Stdio;
use std::{error::Error, path::Path, process::Command};
use sysinfo::{Pid, System};

fn is_daemon_running(pid: u32) -> bool {
    let s = System::new_all();
    if let Some(process) = s.process(Pid::from(pid as usize)) {
        let name = process.name().to_string();
        name.contains("laches_mon")
    } else {
        false
    }
}

pub fn start_monitoring(config_dir: &Path) -> Result<(), Box<dyn Error>> {
    // check if already running
    if let Some(pid) = read_daemon_pid(config_dir) {
        if is_daemon_running(pid) {
            return Err(format!(
                "error: laches_mon is already running (pid: {}). stop it first with `laches stop`",
                pid
            )
            .into());
        }
    }

    let mut exe_path = env::current_exe()?;
    exe_path.pop();
    exe_path.push("laches_mon");

    let instance = Command::new(&exe_path)
        .arg(config_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| {
            format!(
                "error: failed to start laches_mon at '{}': {}",
                exe_path.display(),
                e
            )
        })?;

    let pid = instance.id();
    write_daemon_pid(config_dir, pid)?;
    drop(instance);

    println!("info: started laches_mon daemon (pid: {})", pid);

    Ok(())
}

pub fn stop_monitoring(config_dir: &Path) -> Result<(), Box<dyn Error>> {
    let pid = match read_daemon_pid(config_dir) {
        Some(pid) if is_daemon_running(pid) => pid,
        _ => {
            println!("info: laches_mon is not running");
            clear_daemon_pid(config_dir);
            return Ok(());
        }
    };

    let s = System::new_all();
    if let Some(process) = s.process(Pid::from(pid as usize)) {
        process.kill();
    }
    clear_daemon_pid(config_dir);
    println!("info: stopped laches_mon (pid: {})", pid);

    Ok(())
}

pub fn get_active_processes() -> Vec<Process> {
    let mut active_processes: Vec<Process> = Vec::new();
    let system = System::new_all();

    for process in system.processes().values() {
        let raw_name = process.name().to_string();
        let title = normalize_process_name(&raw_name);

        if title.trim().is_empty() {
            continue;
        }

        let already_tracked = active_processes.iter().any(|p| p.title == title);
        if already_tracked {
            continue;
        }

        let exe_path = process.exe().map(|p| p.to_string_lossy().to_string());
        active_processes.push(Process::with_exe_path(title, exe_path));
    }
    active_processes
}

#[cfg(test)]
mod tests {
    use super::*;

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

        // Each process should be newly created with total usage of 0
        for process in &processes {
            assert_eq!(process.get_total_usage(), 0);
            assert_eq!(process.daily_usage.len(), 0);
            assert_eq!(process.tags.len(), 0);
        }
    }
}
