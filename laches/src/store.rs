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
use uuid::Uuid;

use crate::process_list::ProcessListOptions;

pub const STORE_NAME: &str = "store.json";

#[derive(Deserialize, Serialize, Clone, Tabled)]
pub struct Process {
    pub title: String,
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

/// Get the hostname of the current machine using cross-platform methods
/// No external dependencies required - uses environment variables
pub fn get_hostname() -> String {
    // Try Windows environment variable first
    if let Ok(hostname) = std::env::var("COMPUTERNAME") {
        return hostname;
    }
    // Try Unix/Linux environment variable
    if let Ok(hostname) = std::env::var("HOSTNAME") {
        return hostname;
    }

    // Try reading from /etc/hostname on Unix systems
    #[cfg(unix)]
    {
        if let Ok(hostname) = std::fs::read_to_string("/etc/hostname") {
            let trimmed = hostname.trim();
            if !trimmed.is_empty() {
                return trimmed.to_string();
            }
        }
    }

    "unknown".to_string()
}

pub fn get_machine_id(store_path: &Path) -> String {
    let machine_id_file = store_path.join(".machine_id");

    if let Ok(existing_id) = std::fs::read_to_string(&machine_id_file) {
        let trimmed = existing_id.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }

    let hostname = get_hostname();
    let uuid = Uuid::new_v4();
    let machine_id = format!("{}_{}", hostname, uuid);

    let _ = std::fs::create_dir_all(store_path);
    let _ = std::fs::write(&machine_id_file, &machine_id);

    machine_id
}

impl Process {
    pub fn new(title: String) -> Self {
        let today = Local::now().format("%Y-%m-%d").to_string();
        Self {
            title,
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
        self.last_seen = today;
    }
}

#[derive(Deserialize, Serialize)]
pub struct LachesStore {
    pub daemon_pid: u32,
    pub autostart: bool,      // whether the program runs on startup (yes/no)
    pub update_interval: u64, // how often the list of windows gets updated (seconds)

    // Per-machine process data - key is hostname, value is list of processes for that machine
    #[serde(default)]
    pub machine_data: HashMap<String, Vec<Process>>,

    pub process_list_options: ProcessListOptions,
}

impl Default for LachesStore {
    fn default() -> Self {
        Self {
            autostart: true,
            update_interval: 5,
            machine_data: HashMap::new(),
            daemon_pid: u32::MAX,
            process_list_options: ProcessListOptions::default(),
        }
    }
}

impl LachesStore {
    /// Get processes for the current machine using machine_id from store path
    pub fn get_machine_processes(&self, store_path: &Path) -> Vec<Process> {
        let machine_id = get_machine_id(store_path);
        self.machine_data
            .get(&machine_id)
            .cloned()
            .unwrap_or_default()
    }

    /// Get mutable reference to processes for the current machine using machine_id
    pub fn get_machine_processes_mut(&mut self, store_path: &Path) -> &mut Vec<Process> {
        let machine_id = get_machine_id(store_path);
        self.machine_data.entry(machine_id).or_default()
    }

    /// Get processes for the current machine (uses default path from store location)
    /// For backwards compatibility with existing code
    pub fn get_current_machine_processes(&self) -> Vec<Process> {
        let hostname = get_hostname();
        self.machine_data
            .get(&hostname)
            .cloned()
            .unwrap_or_default()
    }

    /// Get mutable reference to processes for the current machine
    /// For backwards compatibility - prefer get_machine_processes_mut
    pub fn get_current_machine_processes_mut(&mut self) -> &mut Vec<Process> {
        let hostname = get_hostname();
        self.machine_data.entry(hostname).or_default()
    }

    /// Get all processes from all machines (useful for viewing data from all synced machines)
    pub fn get_all_processes(&self) -> Vec<Process> {
        let mut all_processes = Vec::new();
        for processes in self.machine_data.values() {
            all_processes.extend(processes.clone());
        }
        all_processes
    }
}

pub fn get_stored_processes(laches_config: &LachesStore) -> Vec<Process> {
    laches_config.get_current_machine_processes()
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
        assert_eq!(process.get_total_usage(), 0);
        assert_eq!(process.daily_usage.len(), 0);
        assert_eq!(process.tags.len(), 0);
        assert!(!process.last_seen.is_empty());
    }

    #[test]
    fn test_process_add_time() {
        let mut process = Process::new("test_process".to_string());
        process.add_time(100);

        assert_eq!(process.get_total_usage(), 100);
        assert_eq!(process.get_today_usage(), 100);
        assert_eq!(process.get_total_usage(), 100);
    }

    #[test]
    fn test_process_add_time_multiple() {
        let mut process = Process::new("test_process".to_string());
        process.add_time(100);
        process.add_time(50);
        process.add_time(25);

        assert_eq!(process.get_total_usage(), 175);
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
    fn test_get_hostname() {
        let hostname = get_hostname();
        assert!(!hostname.is_empty());
        println!("Detected hostname: {}", hostname);
    }

    #[test]
    fn test_laches_store_default() {
        let store = LachesStore::default();
        assert!(store.autostart);
        assert_eq!(store.update_interval, 5);
        assert_eq!(store.machine_data.len(), 0);
        assert_eq!(store.daemon_pid, u32::MAX);
    }

    #[test]
    fn test_get_stored_processes() {
        let mut store = LachesStore::default();
        let hostname = get_hostname();

        let process1 = Process::new("process1".to_string());
        let process2 = Process::new("process2".to_string());

        store
            .machine_data
            .insert(hostname, vec![process1, process2]);

        let stored = get_stored_processes(&store);
        assert_eq!(stored.len(), 2);
        assert_eq!(stored[0].title, "process1");
        assert_eq!(stored[1].title, "process2");
    }

    #[test]
    fn test_save_and_load_store() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path();
        let hostname = get_hostname();

        let mut store = LachesStore::default();
        store.update_interval = 10;
        let mut process = Process::new("test_process".to_string());
        process.add_time(500);
        store.machine_data.insert(hostname.clone(), vec![process]);

        // Save the store
        save_store(&store, store_path).unwrap();

        // Load the store
        let loaded_store = load_or_create_store(store_path).unwrap();

        assert_eq!(loaded_store.update_interval, 10);
        let processes = loaded_store.machine_data.get(&hostname).unwrap();
        assert_eq!(processes.len(), 1);
        assert_eq!(processes[0].title, "test_process");
        assert_eq!(processes[0].get_total_usage(), 500);
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
        assert!(store.autostart);
    }

    #[test]
    fn test_reset_store() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path();
        let hostname = get_hostname();

        // Create a store with custom data
        let mut store = LachesStore::default();
        store.update_interval = 100;
        store
            .machine_data
            .insert(hostname, vec![Process::new("test".to_string())]);
        save_store(&store, store_path).unwrap();

        // Reset the store
        reset_store(store_path).unwrap();

        // Load and verify it's back to defaults
        let loaded_store = load_or_create_store(store_path).unwrap();
        assert_eq!(loaded_store.update_interval, 5);
        assert_eq!(loaded_store.machine_data.len(), 0);
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
        assert_eq!(deserialized.get_total_usage(), 100);
        assert_eq!(deserialized.tags.len(), 2);
        assert_eq!(deserialized.tags[0], "tag1");
    }

    #[test]
    fn test_multi_machine_storage() {
        let mut store = LachesStore::default();

        // Simulate data from multiple machines
        let machine1_processes = vec![
            Process::new("machine1_process1".to_string()),
            Process::new("machine1_process2".to_string()),
        ];

        let machine2_processes = vec![Process::new("machine2_process1".to_string())];

        store
            .machine_data
            .insert("machine1".to_string(), machine1_processes);
        store
            .machine_data
            .insert("machine2".to_string(), machine2_processes);

        // Verify data is stored separately
        assert_eq!(store.machine_data.len(), 2);
        assert_eq!(store.machine_data.get("machine1").unwrap().len(), 2);
        assert_eq!(store.machine_data.get("machine2").unwrap().len(), 1);

        // Verify get_all_processes returns all
        let all = store.get_all_processes();
        assert_eq!(all.len(), 3);
    }

    #[test]
    fn test_get_current_machine_processes() {
        let mut store = LachesStore::default();
        let hostname = get_hostname();

        let current_machine_processes = vec![
            Process::new("local_process1".to_string()),
            Process::new("local_process2".to_string()),
        ];

        let other_machine_processes = vec![Process::new("remote_process1".to_string())];

        store
            .machine_data
            .insert(hostname.clone(), current_machine_processes);
        store
            .machine_data
            .insert("other_machine".to_string(), other_machine_processes);

        let current = store.get_current_machine_processes();
        assert_eq!(current.len(), 2);
        assert_eq!(current[0].title, "local_process1");
        assert_eq!(current[1].title, "local_process2");
    }

    #[test]
    fn test_get_current_machine_processes_mut() {
        let mut store = LachesStore::default();

        let processes = store.get_current_machine_processes_mut();
        processes.push(Process::new("test_process".to_string()));

        let current = store.get_current_machine_processes();
        assert_eq!(current.len(), 1);
        assert_eq!(current[0].title, "test_process");
    }

    #[test]
    fn test_cross_machine_sync_simulation() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path();

        // Simulate Machine 1 creating and saving data
        let mut store1 = LachesStore::default();
        let mut process1 = Process::new("machine1_app".to_string());
        process1.add_time(100);
        store1
            .machine_data
            .insert("machine1".to_string(), vec![process1]);
        save_store(&store1, store_path).unwrap();

        // Simulate Machine 2 loading, adding its data, and saving
        let mut store2 = load_or_create_store(store_path).unwrap();
        let mut process2 = Process::new("machine2_app".to_string());
        process2.add_time(200);
        store2
            .machine_data
            .insert("machine2".to_string(), vec![process2]);
        save_store(&store2, store_path).unwrap();

        // Simulate Machine 1 loading the synced file
        let store1_reloaded = load_or_create_store(store_path).unwrap();

        // Verify both machines' data is present
        assert_eq!(store1_reloaded.machine_data.len(), 2);
        assert!(store1_reloaded.machine_data.contains_key("machine1"));
        assert!(store1_reloaded.machine_data.contains_key("machine2"));

        let machine1_data = store1_reloaded.machine_data.get("machine1").unwrap();
        assert_eq!(machine1_data[0].title, "machine1_app");
        assert_eq!(machine1_data[0].get_total_usage(), 100);

        let machine2_data = store1_reloaded.machine_data.get("machine2").unwrap();
        assert_eq!(machine2_data[0].title, "machine2_app");
        assert_eq!(machine2_data[0].get_total_usage(), 200);
    }

    #[test]
    fn test_machine_data_isolation() {
        let mut store = LachesStore::default();

        // Add processes to different machines
        let mut proc1 = Process::new("same_app".to_string());
        proc1.add_time(100);
        store
            .machine_data
            .insert("machine1".to_string(), vec![proc1]);

        let mut proc2 = Process::new("same_app".to_string());
        proc2.add_time(200);
        store
            .machine_data
            .insert("machine2".to_string(), vec![proc2]);

        // Verify each machine has its own independent time tracking for same app
        let m1_data = store.machine_data.get("machine1").unwrap();
        let m2_data = store.machine_data.get("machine2").unwrap();

        assert_eq!(m1_data[0].get_total_usage(), 100);
        assert_eq!(m2_data[0].get_total_usage(), 200);
    }

    #[test]
    fn test_machine_id_generation() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path();

        // Generate machine ID first time
        let machine_id1 = get_machine_id(store_path);
        assert!(!machine_id1.is_empty());
        assert!(machine_id1.contains("_")); // Should contain separators

        // Second call should return the same ID (from file)
        let machine_id2 = get_machine_id(store_path);
        assert_eq!(machine_id1, machine_id2);

        // Verify the file was created
        let machine_id_file = store_path.join(".machine_id");
        assert!(machine_id_file.exists());
    }

    #[test]
    fn test_get_machine_processes_with_path() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path();
        let machine_id = get_machine_id(store_path);

        let mut store = LachesStore::default();
        let process1 = Process::new("process1".to_string());
        let process2 = Process::new("process2".to_string());

        store
            .machine_data
            .insert(machine_id, vec![process1, process2]);

        let stored = store.get_machine_processes(store_path);
        assert_eq!(stored.len(), 2);
        assert_eq!(stored[0].title, "process1");
        assert_eq!(stored[1].title, "process2");
    }
}
