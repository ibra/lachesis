use std::{
    error::Error, fs::{self, File, OpenOptions}, io::{BufReader, Write}};

use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use tasklist;
use dirs;

#[derive(Parser)]
#[command(author, version)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Deserialize, Serialize, Clone)]
struct Process {
    title: String,
    uptime: u32,
}

#[derive(Subcommand)]
enum Commands {
    Autostart { toggle: String },
    Start {},
    Stop {},
    List {},
}

#[derive(Deserialize, Serialize)]
struct LachesConfig {
    autostart: bool,      // whether the program runs on startup (yes/no) 
    update_interval: u64, // how often the list of windows gets updated (miliseconds)
    process_information: Vec<Process>  // vector storing all recorded windows    
}

impl Default for LachesConfig {
    fn default() -> Self {
        Self {
            autostart: true,
            update_interval: 10,
            process_information: Vec::new(),
        }
    }
}

const CONFIG_NAME: &str = "config.json";

fn main() {
    use std::process::Command;

    let cli = Cli::parse();
    let mut monitor = Command::new("laches_mon");

    let config = match load_or_create_config(CONFIG_NAME) {
        Ok(config) => config,
        Err(error) => panic!("error: failed to load config file: {}", error)
    };

    match &cli.command {
        Commands::Autostart { toggle } => {
            if toggle == "yes" {
                println!("info: enabled boot on startup.")
            } else if toggle == "no" {
                println!("info: disabled boot on startup.")
            }
        }

        Commands::Start {} => {
            let active_windows = get_active_processes();
            println!("started monitoring {} windows", &active_windows.len());

            monitor
                .args(["/C", format!("{}", config.update_interval).as_str()])
                .spawn()
                .expect("error: failed to execute laches_mon (monitoring daemon)");
        }

        Commands::List {} => {
            let all_windows = get_all_processes(&config);
            for window in &all_windows {
                println!("{} | {} seconds", window.title, window.uptime);
            }

            if all_windows.is_empty() {
                println!("warning: no  windows.")
            }
        }

        Commands::Stop {} => {
            println!("info: attempting to kill daemon");
            // todo: kill daemon
        }
    }
}

fn load_or_create_config(filename: &str) -> Result<LachesConfig, Box<dyn Error>> {
    let config_path = dirs::config_dir().unwrap().join("./lachesis");

    if !&config_path.join(filename).exists() {
        fs::create_dir_all(&config_path).expect("Failed to create directories");

        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .open(&config_path.join(filename))?;

        let config_json = serde_json::to_string(&LachesConfig::default())?;
        file.write_all(config_json.as_bytes())?;
    }

    let file = File::open(&config_path.join(filename))?;
    let reader = BufReader::new(file);
    let laches_config = serde_json::from_reader(reader)?;

    Ok(laches_config)
}

fn get_all_processes(laches_config: &LachesConfig) -> Vec<Process> {
    let mut all_processes: Vec<Process> = Vec::new();

    for process in  &laches_config.process_information {
       all_processes.push(process.clone()); 
    }

    all_processes
}

fn get_active_processes() -> Vec<Process> {
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
