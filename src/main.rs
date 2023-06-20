use std::collections::HashMap;

use clap::{Parser, Subcommand};
use tasklist;
use windows::core::*;
use windows::{Win32::Foundation::*, Win32::UI::WindowsAndMessaging::*};

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

#[cfg(windows)]
fn main() -> Result<()> {
    let cli = Cli::parse();
    let mut active_windows: HashMap<String, i32> = HashMap::new();

    match &cli.command {
        Commands::Autostart { toggle } => {
            if toggle == "on" {
                println!("Atropos will now boot on startup!")
            } else if toggle == "off" {
                println!("Stopped Atropos from booting on startup!")
            }
        }
        Commands::Start {} => {
            active_windows = get_active_processes();
            println!("Started monitoring {} windows", active_windows.keys().len());
        }
        Commands::List {} => {}
        Commands::Stop {} => {}
    }
    Ok(())
}

fn get_active_processes() -> HashMap<String, i32> {
    let mut windows: HashMap<String, i32> = HashMap::new();
    for i in unsafe { tasklist::Tasklist::new() } {
        let cpn = match i.get_file_info().get("CompanyName") {
            Some(h) => h.to_string(),
            None => "".to_string(),
        };
        let des = match i.get_file_info().get("FileDescription") {
            Some(h) => h.to_string(),
            None => "".to_string(),
        };
        windows.insert(des, 0);
    }
    windows
}
