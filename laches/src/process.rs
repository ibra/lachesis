use crate::store::{LachesStore, Process};
use sysinfo::System;

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

pub fn get_stored_processes(laches_config: &LachesStore) -> Vec<Process> {
    let mut stored_processes: Vec<Process> = Vec::new();

    for process in &laches_config.process_information {
        stored_processes.push(process.clone());
    }

    stored_processes
}
