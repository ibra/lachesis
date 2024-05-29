use serde::{Deserialize, Serialize};
use tasklist;

#[derive(Deserialize, Serialize, Clone)]
pub struct Process {
    pub title: String,
    pub uptime: u32,
}

#[derive(Deserialize, Serialize)]
pub struct LachesStore {
    pub autostart: bool,      // whether the program runs on startup (yes/no)
    pub update_interval: u64, // how often the list of windows gets updated (miliseconds)
    pub process_information: Vec<Process>, // vector storing all recorded windows
}

impl Default for LachesStore {
    fn default() -> Self {
        Self {
            autostart: true,
            update_interval: 10,
            process_information: Vec::new(),
        }
    }
}

pub fn get_active_processes() -> Vec<Process> {
    let mut active_processes: Vec<Process> = Vec::new();

    for process in unsafe { tasklist::Tasklist::new() } {
        let name = match process.get_file_info().get("ProductName") {
            Some(h) => h.to_string(),
            None => "".to_string(),
        };

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

pub fn get_all_processes(laches_config: &LachesStore) -> Vec<Process> {
    let mut all_processes: Vec<Process> = Vec::new();

    for process in &laches_config.process_information {
        all_processes.push(process.clone());
    }

    all_processes
}
