use chrono::Local;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    error::Error,
    fs::{self, File, OpenOptions},
    io::{BufReader, Write},
    path::Path,
};
use tabled::Tabled;

use crate::process_list::ProcessListOptions;

pub const STORE_NAME: &str = "store.json";

#[derive(Deserialize, Serialize, Clone, Tabled)]
pub struct Process {
    pub title: String,
    #[tabled(skip)]
    #[serde(default)]
    pub uptime: u64,
    #[tabled(skip)]
    #[serde(default)]
    pub daily_usage: HashMap<String, u64>,
    #[tabled(skip)]
    #[serde(default)]
    pub tags: Vec<String>,
    #[tabled(skip)]
    #[serde(default = "get_today_date")]
    pub last_seen: String,
}

fn get_today_date() -> String {
    Local::now().format("%Y-%m-%d").to_string()
}

impl Process {
    pub fn new(title: String) -> Self {
        let today = Local::now().format("%Y-%m-%d").to_string();
        Self {
            title,
            uptime: 0,
            daily_usage: HashMap::new(),
            tags: Vec::new(),
            last_seen: today,
        }
    }

    pub fn get_today_usage(&self) -> u64 {
        let today = Local::now().format("%Y-%m-%d").to_string();
        *self.daily_usage.get(&today).unwrap_or(&0)
    }

    pub fn get_total_usage(&self) -> u64 {
        self.daily_usage.values().sum()
    }

    pub fn add_time(&mut self, seconds: u64) {
        let today = Local::now().format("%Y-%m-%d").to_string();
        let current = self.daily_usage.get(&today).unwrap_or(&0);
        self.daily_usage.insert(today.clone(), current + seconds);
        self.uptime += seconds;
        self.last_seen = today;
    }
}

#[derive(Deserialize, Serialize)]
pub struct LachesStore {
    pub daemon_pid: u32,
    pub autostart: bool,      // whether the program runs on startup (yes/no)
    pub update_interval: u64, // how often the list of windows gets updated (seconds)
    pub process_information: Vec<Process>, // vector storing all recorded windows
    pub process_list_options: ProcessListOptions,
}

impl Default for LachesStore {
    fn default() -> Self {
        Self {
            autostart: true,
            update_interval: 5,
            process_information: Vec::new(),
            daemon_pid: u32::MAX,
            process_list_options: ProcessListOptions::default(),
        }
    }
}

pub fn get_stored_processes(laches_config: &LachesStore) -> Vec<Process> {
    let mut stored_processes: Vec<Process> = Vec::new();

    for process in &laches_config.process_information {
        stored_processes.push(process.clone());
    }

    stored_processes
}

pub fn save_store(store: &LachesStore, store_path: &Path) -> Result<(), Box<dyn Error>> {
    let file_path = store_path.join(STORE_NAME);
    let mut file = File::create(file_path)?;

    let laches_store = serde_json::to_string(store)?;
    file.write_all(laches_store.as_bytes())?;

    Ok(())
}

pub fn load_or_create_store(store_path: &Path) -> Result<LachesStore, Box<dyn Error>> {
    if !&store_path.join(STORE_NAME).exists() {
        fs::create_dir_all(store_path).expect("error: failed to create directories");

        let mut file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(store_path.join(STORE_NAME))?;

        let laches_store = serde_json::to_string(&LachesStore::default())?;
        println!("info: created default configuration file");
        file.write_all(laches_store.as_bytes())?;
    }

    let file = File::open(store_path.join(STORE_NAME))?;
    let reader = BufReader::new(file);
    let laches_store = serde_json::from_reader(reader)?;

    Ok(laches_store)
}

pub fn reset_store(store_path: &Path) -> std::io::Result<()> {
    fs::remove_file(store_path.join(STORE_NAME))?;
    load_or_create_store(store_path).expect("error: failed to create default config file");

    Ok(())
}
