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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_process_new() {
        let process = Process::new("test_process".to_string());
        assert_eq!(process.title, "test_process");
        assert_eq!(process.uptime, 0);
        assert_eq!(process.daily_usage.len(), 0);
        assert_eq!(process.tags.len(), 0);
        assert!(!process.last_seen.is_empty());
    }

    #[test]
    fn test_process_add_time() {
        let mut process = Process::new("test_process".to_string());
        process.add_time(100);

        assert_eq!(process.uptime, 100);
        assert_eq!(process.get_today_usage(), 100);
        assert_eq!(process.get_total_usage(), 100);
    }

    #[test]
    fn test_process_add_time_multiple() {
        let mut process = Process::new("test_process".to_string());
        process.add_time(100);
        process.add_time(50);
        process.add_time(25);

        assert_eq!(process.uptime, 175);
        assert_eq!(process.get_today_usage(), 175);
        assert_eq!(process.get_total_usage(), 175);
    }

    #[test]
    fn test_process_get_today_usage_zero() {
        let process = Process::new("test_process".to_string());
        assert_eq!(process.get_today_usage(), 0);
    }

    #[test]
    fn test_process_get_total_usage_with_multiple_days() {
        let mut process = Process::new("test_process".to_string());
        let today = Local::now().format("%Y-%m-%d").to_string();
        let yesterday = (Local::now() - chrono::Duration::days(1))
            .format("%Y-%m-%d")
            .to_string();

        process.daily_usage.insert(today, 100);
        process.daily_usage.insert(yesterday, 200);

        assert_eq!(process.get_total_usage(), 300);
    }

    #[test]
    fn test_laches_store_default() {
        let store = LachesStore::default();
        assert_eq!(store.autostart, true);
        assert_eq!(store.update_interval, 5);
        assert_eq!(store.process_information.len(), 0);
        assert_eq!(store.daemon_pid, u32::MAX);
    }

    #[test]
    fn test_get_stored_processes() {
        let mut store = LachesStore::default();
        let process1 = Process::new("process1".to_string());
        let process2 = Process::new("process2".to_string());

        store.process_information.push(process1);
        store.process_information.push(process2);

        let stored = get_stored_processes(&store);
        assert_eq!(stored.len(), 2);
        assert_eq!(stored[0].title, "process1");
        assert_eq!(stored[1].title, "process2");
    }

    #[test]
    fn test_save_and_load_store() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path();

        let mut store = LachesStore::default();
        store.update_interval = 10;
        let mut process = Process::new("test_process".to_string());
        process.add_time(500);
        store.process_information.push(process);

        // Save the store
        save_store(&store, store_path).unwrap();

        // Load the store
        let loaded_store = load_or_create_store(store_path).unwrap();

        assert_eq!(loaded_store.update_interval, 10);
        assert_eq!(loaded_store.process_information.len(), 1);
        assert_eq!(loaded_store.process_information[0].title, "test_process");
        assert_eq!(loaded_store.process_information[0].uptime, 500);
    }

    #[test]
    fn test_load_or_create_store_creates_new() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path();

        // Store doesn't exist yet
        assert!(!store_path.join(STORE_NAME).exists());

        let store = load_or_create_store(store_path).unwrap();

        // Store should now exist with default values
        assert!(store_path.join(STORE_NAME).exists());
        assert_eq!(store.update_interval, 5);
        assert_eq!(store.autostart, true);
    }

    #[test]
    fn test_reset_store() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path();

        // Create a store with custom data
        let mut store = LachesStore::default();
        store.update_interval = 100;
        store
            .process_information
            .push(Process::new("test".to_string()));
        save_store(&store, store_path).unwrap();

        // Reset the store
        reset_store(store_path).unwrap();

        // Load and verify it's back to defaults
        let loaded_store = load_or_create_store(store_path).unwrap();
        assert_eq!(loaded_store.update_interval, 5);
        assert_eq!(loaded_store.process_information.len(), 0);
    }

    #[test]
    fn test_process_serialization() {
        let mut process = Process::new("test_process".to_string());
        process.add_time(100);
        process.tags.push("tag1".to_string());
        process.tags.push("tag2".to_string());

        let serialized = serde_json::to_string(&process).unwrap();
        let deserialized: Process = serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized.title, "test_process");
        assert_eq!(deserialized.uptime, 100);
        assert_eq!(deserialized.tags.len(), 2);
        assert_eq!(deserialized.tags[0], "tag1");
    }
}
