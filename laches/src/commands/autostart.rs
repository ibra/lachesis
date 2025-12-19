use auto_launch::AutoLaunch;
use std::{error::Error, path::Path};

use crate::store::{load_or_create_store, STORE_NAME};

pub fn handle_autostart(toggle: &str, store_path: &Path) -> Result<(), Box<dyn Error>> {
    let store_file = store_path.join(STORE_NAME);
    let laches_store = load_or_create_store(store_path)?;

    let laches_mon_path = if cfg!(windows) {
        std::env::current_exe()?
            .parent()
            .ok_or("Failed to get parent directory")?
            .join("laches_mon.exe")
    } else {
        std::env::current_exe()?
            .parent()
            .ok_or("Failed to get parent directory")?
            .join("laches_mon")
    };

    if !laches_mon_path.exists() {
        return Err(format!(
            "error: laches_mon executable not found at: {}",
            laches_mon_path.display()
        )
        .into());
    }

    let args = vec![
        laches_store.update_interval.to_string(),
        store_file.to_string_lossy().to_string(),
    ];

    let auto = AutoLaunch::new(
        "laches_mon",
        laches_mon_path.to_str().ok_or("Invalid path")?,
        &args,
    );

    match toggle {
        "yes" => {
            if auto.is_enabled()? {
                println!("info: autostart is already enabled.");
            } else {
                auto.enable()?;
                println!("info: enabled laches_mon to run at startup.");
            }
        }
        "no" => {
            if !auto.is_enabled()? {
                println!("info: autostart is already disabled.");
            } else {
                auto.disable()?;
                println!("info: disabled laches_mon from running at startup.");
            }
        }
        _ => {
            return Err("error: invalid option for autostart. Use 'yes' or 'no'.".into());
        }
    }

    Ok(())
}
