use clap::Parser;
use laches::{
    cli::{Cli, Commands},
    process::{start_monitoring, stop_monitoring},
    process_list::ListMode,
    store::{
        get_stored_processes, load_or_create_store, reset_store, save_store, LachesStore,
        STORE_NAME,
    },
    utils::{confirm, format_uptime},
};
use std::{error::Error, path::Path, process::Command};
use tabled::{builder::Builder, settings::Style};

fn main() -> Result<(), Box<dyn Error>> {
    let store_path = match dirs::config_dir() {
        Some(dir) => dir.join("lachesis"),
        None => return Err("error: failed to get configuration directory".into()),
    };
    std::fs::create_dir_all(&store_path)?;

    let mut laches_store = load_or_create_store(&store_path)?;
    configure_daemon(&laches_store, &store_path);

    let cli = Cli::parse();

    match &cli.command {
        Commands::Autostart { toggle } => handle_autostart(toggle),
        Commands::Start => start_monitoring(&mut laches_store, &store_path),
        Commands::Stop => stop_monitoring(&mut laches_store),
        Commands::Mode { mode } => set_mode(mode, &mut laches_store),
        Commands::List => list_processes(&laches_store),
        Commands::Reset => confirm_reset_store(&store_path),
    }?;

    save_store(&laches_store, &store_path)?;

    Ok(())
}

fn configure_daemon(laches_store: &LachesStore, store_path: &Path) {
    let mut monitor = Command::new("laches_mon");
    monitor
        .arg(&laches_store.update_interval.to_string())
        .arg(&store_path.join(STORE_NAME));
}

fn handle_autostart(toggle: &str) -> Result<(), Box<dyn Error>> {
    match toggle {
        "yes" => println!("info: enabled boot on startup."),
        "no" => println!("info: disabled boot on startup."),
        _ => println!("error: invalid option for autostart. Use 'yes' or 'no'."),
    }
    todo!("info: command not yet implemented.")
}

fn set_mode(mode: &str, laches_store: &mut LachesStore) -> Result<(), Box<dyn Error>> {
    match mode.parse::<ListMode>() {
        Ok(variant) => {
            laches_store.process_list_options.mode = variant;
            println!(
                "info: mode set to: {}",
                laches_store.process_list_options.mode.to_str()
            );
            Ok(())
        }
        Err(_) => Err(format!("error: no match found for mode: '{}'", mode).into()),
    }
}

fn list_processes(laches_store: &LachesStore) -> Result<(), Box<dyn Error>> {
    let all_windows = get_stored_processes(laches_store);
    let mut builder = Builder::default();

    println!(
        "{}",
        &format!(
            "Tracked Window Usage ({} Mode)",
            match laches_store.process_list_options.mode {
                ListMode::Whitelist => "Whitelist",
                ListMode::Blacklist => "Blacklist",
                ListMode::Default => "Default",
            }
        )
    );

    let mut sorted_windows = all_windows.clone();
    sorted_windows.sort_by_key(|window| std::cmp::Reverse(window.uptime));

    builder.push_record(["Process Name", "Usage Time"]);

    for window in &sorted_windows {
        match laches_store.process_list_options.mode {
            ListMode::Whitelist => {
                let whitelist = laches_store
                    .process_list_options
                    .whitelist
                    .as_deref()
                    .unwrap_or(&[]);
                if !whitelist.contains(&window.title) {
                    continue;
                }
            }
            ListMode::Blacklist => {
                let blacklist = laches_store
                    .process_list_options
                    .blacklist
                    .as_deref()
                    .unwrap_or(&[]);
                if blacklist.contains(&window.title) {
                    continue;
                }
            }
            ListMode::Default => {
                // default mode does no filtering, so process all windows
            }
        }

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

fn confirm_reset_store(store_path: &Path) -> Result<(), Box<dyn Error>> {
    if confirm("are you sure you want to wipe the current store? [y/N]") {
        reset_store(store_path).expect("error: failed to reset store file");
    } else {
        println!("info: aborted reset operation");
    }

    Ok(())
}
