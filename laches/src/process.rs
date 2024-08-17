use std::{error::Error, path::Path, process::Command};

use crate::{
    store::{LachesStore, Process, STORE_NAME},
    utils::confirm,
};
use sysinfo::{Pid, System};

pub fn start_monitoring(
    laches_store: &mut LachesStore,
    store_path: &Path,
) -> Result<(), Box<dyn Error>> {
    let active_windows = get_active_processes();
    println!("info: started monitoring {} windows", &active_windows.len());

    let mut monitor = Command::new("laches_mon");
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

        active_processes.push(Process {
            title: name,
            uptime: 0,
        });
    }
    active_processes
}
