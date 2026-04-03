use crate::store::{LachesStore, Process, STORE_NAME};
use std::env;
use std::process::Stdio;
use std::{error::Error, path::Path, process::Command};
use sysinfo::{Pid, System};

fn is_daemon_running(pid: u32) -> bool {
    if pid == u32::MAX {
        return false;
    }
    let s = System::new_all();
    if let Some(process) = s.process(Pid::from(pid as usize)) {
        let name = process.name().to_string();
        name.contains("laches_mon")
    } else {
        false
    }
}

pub fn start_monitoring(
    laches_store: &mut LachesStore,
    store_path: &Path,
) -> Result<(), Box<dyn Error>> {
    if is_daemon_running(laches_store.daemon_pid) {
        return Err(format!(
            "error: laches_mon is already running (pid: {}). stop it first with `laches stop`",
            laches_store.daemon_pid
        )
        .into());
    }

    let mut exe_path = env::current_exe()?;
    exe_path.pop();
    exe_path.push("laches_mon");

    let instance = Command::new(&exe_path)
        .arg(laches_store.update_interval.to_string())
        .arg(store_path.join(STORE_NAME))
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
    laches_store.daemon_pid = pid;
    // child handle is dropped here - on both windows and unix this does NOT
    // kill the child process, it just releases our handle to it
    drop(instance);

    println!("info: started laches_mon daemon (pid: {})", pid);

    Ok(())
}

pub fn stop_monitoring(laches_store: &mut LachesStore) -> Result<(), Box<dyn Error>> {
    if !is_daemon_running(laches_store.daemon_pid) {
        println!("info: laches_mon is not running");
        laches_store.daemon_pid = u32::MAX;
        return Ok(());
    }

    let s = System::new_all();
    if let Some(process) = s.process(Pid::from(laches_store.daemon_pid as usize)) {
        process.kill();
    }
    println!(
        "info: stopped laches_mon (pid: {})",
        laches_store.daemon_pid
    );
    laches_store.daemon_pid = u32::MAX;

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
