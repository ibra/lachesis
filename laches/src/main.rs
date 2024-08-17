use clap::Parser;
use laches::{
    cli::{Cli, Commands},
    process::{start_monitoring, stop_monitoring},
    store::{get_stored_processes, load_or_create_store, reset_store, LachesStore, STORE_NAME},
    utils::{confirm, format_uptime},
};
use std::{error::Error, process::Command};
use tabled::{builder::Builder, settings::Style};

fn main() -> Result<(), Box<dyn Error>> {
    let store_path = dirs::config_dir().unwrap().join("lachesis");
    let mut laches_store = load_or_create_store(&store_path)?;

    let cli = Cli::parse();

    configure_daemon(&laches_store, &store_path);

    match &cli.command {
        Commands::Autostart { toggle } => handle_autostart(toggle),
        Commands::Start => start_monitoring(&mut laches_store, &store_path),
        Commands::Stop => stop_monitoring(&mut laches_store),
        Commands::List => list_windows(&laches_store),
        Commands::Reset => confirm_reset_store(&store_path),
    }?;

    Ok(())
}

fn configure_daemon(laches_store: &LachesStore, store_path: &std::path::PathBuf) {
    let mut monitor = Command::new("laches_mon");
    monitor
        .arg(&laches_store.update_interval.to_string())
        .arg(&store_path.join(STORE_NAME));
}

fn confirm_reset_store(store_path: &std::path::PathBuf) -> Result<(), Box<dyn Error>> {
    if confirm("are you sure you want to wipe the current store? [y/N]") {
        reset_store(&store_path).expect("error: failed to reset store file");
    } else {
        println!("info: aborted reset operation");
    }

    Ok(())
}

fn handle_autostart(toggle: &str) -> Result<(), Box<dyn Error>> {
    match toggle {
        "yes" => println!("info: enabled boot on startup."),
        "no" => println!("info: disabled boot on startup."),
        _ => println!("error: invalid option for autostart. Use 'yes' or 'no'."),
    }
    todo!("info: command not yet implemented.")
}

// todo: blacklisting/whitelisting, categories, tagging
fn list_windows(laches_store: &LachesStore) -> Result<(), Box<dyn Error>> {
    let all_windows = get_stored_processes(&laches_store);
    let mut builder = Builder::default();

    let mut sorted_windows = all_windows.clone();
    sorted_windows.sort_by_key(|window| std::cmp::Reverse(window.uptime));

    builder.push_record(["Process Name", "Usage Time"]);

    for window in &sorted_windows {
        builder.push_record([&window.title, &format_uptime(window.uptime)]);
    }

    let mut table = builder.build();
    table.with(Style::rounded());

    print!("{}", table);

    if all_windows.is_empty() {
        println!("warning: no monitored windows");
    }

    Ok(())
}
