use serde::{Deserialize, Serialize};
use std::{
    error::Error,
    fs::{self, File, OpenOptions},
    io::{BufReader, Write},
    path::{Path, PathBuf},
};
use tabled::Tabled;

pub const STORE_NAME: &str = "store.json";

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

pub fn load_or_create_store(store_path: &PathBuf) -> Result<LachesStore, Box<dyn Error>> {
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

pub fn reset_store(store_path: &PathBuf) -> std::io::Result<()> {
    fs::remove_file(store_path.join(STORE_NAME))?;
    load_or_create_store(store_path).expect("error: failed to create default config file");

    Ok(())
}
