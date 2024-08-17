use serde::{Deserialize, Serialize};
use tabled::Tabled;

#[derive(Deserialize, Serialize, Clone, Tabled)]
pub struct Process {
    pub title: String,
    pub uptime: u64,
}

#[derive(Deserialize, Serialize)]
pub struct LachesStore {
    pub daemon_pid: u32,
    pub autostart: bool,      // whether the program runs on startup (yes/no)
    pub update_interval: u64, // how often the list of windows gets updated (seconds)
    pub process_information: Vec<Process>, // vector storing all recorded windows
}

impl Default for LachesStore {
    fn default() -> Self {
        Self {
            autostart: true,
            update_interval: 1,
            process_information: Vec::new(),
            daemon_pid: u32::MAX,
        }
    }
}
