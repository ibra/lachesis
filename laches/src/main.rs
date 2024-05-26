use std::{
    error::Error,
    fs::{File, OpenOptions},
    io::{BufReader, Read, Write},
    path::Path,
};

use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use tasklist;

#[derive(Parser)]
#[command(author, version)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

struct ActiveWindow {
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

#[derive(Debug, Deserialize, Serialize)]
struct LachesConfig {
    autostart: bool,      // whether the program runs on startup in seconds
    update_interval: u32, // how often the list of windows gets updated, in miliseconds
}

impl Default for LachesConfig {
    fn default() -> Self {
        Self {
            autostart: true,
            update_interval: 10,
        }
    }
}

const CONFIG_PATH: &str = "%PROGRAMDATA%\\lachesis\\config\\%P%";

fn main() {
    use std::process::Command;

    let cli = Cli::parse();
    let mut active_windows: Vec<ActiveWindow> = Vec::new();

    let mut monitor = Command::new("");

    let config = match load_or_create_config("config.json") {
        Ok(config) => config,
        Err(_) => panic!("Error encountered while attempting to load config file."),
    };

    match &cli.command {
        Commands::Autostart { toggle } => {
            if toggle == "on" {
                println!("enabled boot on startup.")
            } else if toggle == "off" {
                println!("disabled boot on startup.")
            }
        }

        Commands::Start {} => {
            active_windows = get_active_processes();
            println!("started monitoring {} windows", active_windows.len());

            monitor
                .args(["/C", "start", "ls", "arguments"])
                .spawn()
                .expect("failed to execute laches_mon (monitoring application).");
        }

        Commands::List {} => {
            active_windows = get_active_processes();
            for window in active_windows {
                println!("{} | {} seconds", window.title, window.uptime);
            }
        }

        Commands::Stop {} => {
            println!("Stopping window tracking.");
        }
    }
}

fn load_or_create_config(filename: &str) -> Result<LachesConfig, Box<dyn Error>> {
    let config_path = Path::new(filename);

    if !config_path.exists() {
        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .open(config_path)?;

        let config_json = serde_json::to_string(&LachesConfig::default())?;
        file.write_all(config_json.as_bytes())?;
    }

    let file = File::open(filename)?;

    let reader = BufReader::new(file);
    let laches_config = serde_json::from_reader(reader)?;

    Ok(laches_config)
}

fn get_active_processes() -> Vec<ActiveWindow> {
    let mut active_windows: Vec<ActiveWindow> = Vec::new();

    for i in unsafe { tasklist::Tasklist::new() } {
        let name = match i.get_file_info().get("ProductName") {
            Some(h) => h.to_string(),
            None => "".to_string(),
        };

        let contains_title = active_windows.iter().any(|window| window.title == name);

        if name.trim() == "" || contains_title {
            continue;
        }

        active_windows.push(ActiveWindow {
            title: name,
            uptime: 0,
        });
    }
    active_windows
}
