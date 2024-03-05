use std::fs::File;
use std::io::{BufRead, Write};
use std::{fs, io};

use clap::{Parser, Subcommand};
use tabled::{Table, Tabled};
use tasklist;
use windows::core::*;

#[derive(Parser)]
#[command(author, version)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

struct ActiveWindow {
    title: String,
    uptime: u64,
}

#[derive(Subcommand)]
enum Commands {
    Autostart { toggle: String },
    Start {},
    Stop {},
    List {},
}

#[cfg(windows)]
fn main() -> Result<()> {
    use std::process::Command;

    let cli = Cli::parse();
    let mut config = fs::read_to_string("config.ini");
    let mut active_windows: Vec<ActiveWindow> = Vec::new();

    let mut monitor = Command::new("cmd");

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
                .expect("Failed to execute second program");
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

    Ok(())
}

fn get_config() -> io::Result<String> {
    let file_path = "config.ini";

    if fs::metadata(&file_path).is_ok() {
        let mut file = File::create(&file_path);
    }

    let file = File::open(&file_path)?;
    let mut lines = io::BufReader::new(file).lines();

    if let Some(Ok(first_line)) = lines.next() {
        Ok(first_line)
    } else {
        create_config();
        Ok("whoopsies".to_string())
    }
}

fn create_config() {}

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
