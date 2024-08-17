use clap::Parser;
use laches::{
    cli::{Cli, Commands},
    process::get_active_processes,
    store::{get_stored_processes, load_or_create_store, reset_store, save_store, STORE_NAME},
    utils::{confirm, format_uptime},
};
use std::error::Error;
use sysinfo::{Pid, System};
use tabled::{builder::Builder, settings::Style};

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
            println!("info: started monitoring {} windows", &active_windows.len());

            let instance = monitor
                .spawn()
                .expect("error: failed to execute laches_mon (monitoring daemon)");

            laches_store.daemon_pid = instance.id();
            save_store(&laches_store, &store_path)?;
        }

        Commands::List {} => {
            // todo: add blacklisting/whitelisting
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
