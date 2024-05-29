use clap::{Parser, Subcommand};
use dirs;
use laches::{get_active_processes, get_all_processes, LachesStore, Process};
use std::{
    error::Error,
    fs::{self, File, OpenOptions},
    io::{BufReader, Write},
    path::PathBuf,
};

#[derive(Parser)]
#[command(author, version)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Autostart { toggle: String },
    Start {},
    Stop {},
    List {},
}

const STORE_NAME: &str = "store.json";

fn main() {
    use std::process::Command;

    let cli = Cli::parse();
    let mut monitor = Command::new("laches_mon");

    let store_path = dirs::config_dir().unwrap().join("lachesis");

    let laches_store = match load_or_create_config(STORE_NAME, &store_path) {
        Ok(laches_store) => laches_store,
        Err(error) => panic!("error: failed to load config file: {}", error),
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
                .arg(&laches_store.update_interval.to_string())
                .arg(&store_path.join(STORE_NAME))
                .spawn()
                .expect("error: failed to execute laches_mon (monitoring daemon)");
        }

        Commands::List {} => {
            let all_windows = get_all_processes(&laches_store);
            for window in &all_windows {
                println!("{} | {} seconds", window.title, window.uptime);
            }

            if all_windows.is_empty() {
                println!("warning: no monitored windows.")
            }
        }

        Commands::Stop {} => {
            println!("info: attempting to kill daemon");
        }
    }
}

fn load_or_create_config(
    file_name: &str,
    file_path: &PathBuf,
) -> Result<LachesStore, Box<dyn Error>> {
    if !&file_path.join(file_name).exists() {
        fs::create_dir_all(&file_path).expect("error: failed to create directories");

        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .open(&file_path.join(file_name))?;

        let laches_store = serde_json::to_string(&LachesStore::default())?;
        file.write_all(laches_store.as_bytes())?;
    }

    let file = File::open(&file_path.join(file_name))?;
    let reader = BufReader::new(file);
    let laches_store = serde_json::from_reader(reader)?;

    Ok(laches_store)
}
