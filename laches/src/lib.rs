use serde::{Serialize,Deserialize};

#[derive(Deserialize, Serialize, Clone)]
pub struct Process {
    pub title: String,
    pub uptime: u32,
}

#[derive(Deserialize, Serialize)]
pub struct LachesStore {
    pub autostart: bool,      // whether the program runs on startup (yes/no) 
    pub update_interval: u64, // how often the list of windows gets updated (miliseconds)
    pub process_information: Vec<Process>  // vector storing all recorded windows    
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