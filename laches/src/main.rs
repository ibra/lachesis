use clap::{Parser, Subcommand};
use laches::{get_active_processes, get_stored_processes, LachesStore};
use std::{
    error::Error,
    fs::{self, File, OpenOptions},
    io::{self, BufReader, Write},
    path::{Path, PathBuf},
};
use sysinfo::{Pid, System};
use tabled::{builder::Builder, settings::Style};

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
    Start,
    Stop,
    List,
    Reset,
}

const STORE_NAME: &str = "store.json";

fn main() -> Result<(), Box<dyn Error>> {
    use std::process::Command;

    let store_path = dirs::config_dir().unwrap().join("lachesis");

    let mut laches_store = match load_or_create_store(&store_path) {
        Ok(laches_store) => laches_store,
        Err(error) => panic!("error: failed to load config file: {}", error),
    };

    let cli = Cli::parse();

    let mut monitor = Command::new("laches_mon");
    monitor
        .arg(&laches_store.update_interval.to_string())
        .arg(&store_path.join(STORE_NAME));

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

            let instance = monitor
                .spawn()
                .expect("error: failed to execute laches_mon (monitoring daemon)");

            laches_store.daemon_pid = instance.id();
            save_store(&laches_store, &store_path)?;
        }

        Commands::List {} => {
            let all_windows = get_stored_processes(&laches_store);
            let mut builder = Builder::default();

            builder.push_record(["Window", "Usage Time"]);

            for window in &all_windows {
                builder.push_record([&window.title, &format_uptime(window.uptime)]);
            }

            let mut table = builder.build();
            table.with(Style::rounded());

            print!("{}", table);

            if all_windows.is_empty() {
                println!("warning: no monitored windows");
            }
        }

        Commands::Stop {} => {
            if confirm("are you sure you want to stop window tracking (kill laches_mon)? [y/N]") {
                let s = System::new_all();
                if let Some(process) = s.process(Pid::from(laches_store.daemon_pid as usize)) {
                    process.kill();
                }
                println!("info: killed laches_mon (monitoring daemon)");
            } else {
                println!("info: aborted stop operation");
            }
        }

        Commands::Reset {} => {
            if confirm("are you sure you want to wipe the current store? [y/N]") {
                reset_store(&store_path).expect("error: failed to reset store file");
            } else {
                println!("info: aborted reset operation");
            }
        }
    }

    Ok(())
}

fn format_uptime(seconds: u64) -> String {
    let days = seconds / 86400;
    let hours = (seconds % 86400) / 3600;
    let minutes = (seconds % 3600) / 60;
    let seconds = seconds % 60;

    if days > 0 {
        format!("{}d {}h {}m {}s", days, hours, minutes, seconds)
    } else if hours > 0 {
        format!("{}h {}m {}s", hours, minutes, seconds)
    } else if minutes > 0 {
        format!("{}m {}s", minutes, seconds)
    } else {
        format!("{}s", seconds)
    }
}

fn confirm(prompt: &str) -> bool {
    print!("{}", prompt);
    io::stdout().flush().unwrap();

    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();

    matches!(input.trim().to_lowercase().as_str(), "y" | "yes")
}

fn save_store(store: &LachesStore, store_path: &Path) -> Result<(), Box<dyn Error>> {
    let file_path = store_path.join(STORE_NAME);
    let mut file = File::create(file_path)?;

    let laches_store = serde_json::to_string(store)?;
    file.write_all(laches_store.as_bytes())?;

    Ok(())
}

fn load_or_create_store(store_path: &PathBuf) -> Result<LachesStore, Box<dyn Error>> {
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

fn reset_store(store_path: &PathBuf) -> std::io::Result<()> {
    fs::remove_file(store_path.join(STORE_NAME))?;
    load_or_create_store(store_path).expect("error: failed to create default config file");

    Ok(())
}
