use auto_launch::AutoLaunch;
use std::{error::Error, path::Path};

pub fn handle_autostart(toggle: &str, config_dir: &Path) -> Result<(), Box<dyn Error>> {
    let laches_mon_path = if cfg!(windows) {
        std::env::current_exe()?
            .parent()
            .ok_or("failed to get parent directory")?
            .join("laches_mon.exe")
    } else {
        std::env::current_exe()?
            .parent()
            .ok_or("failed to get parent directory")?
            .join("laches_mon")
    };

    if !laches_mon_path.exists() {
        return Err(format!(
            "error: laches_mon executable not found at: {}",
            laches_mon_path.display()
        )
        .into());
    }

    // the daemon now takes the config directory as its only argument
    let args = vec![config_dir.to_string_lossy().to_string()];

    #[cfg(target_os = "macos")]
    let auto = AutoLaunch::new(
        "laches_mon",
        laches_mon_path.to_str().ok_or("invalid path")?,
        &args,
        false,
    );

    #[cfg(not(target_os = "macos"))]
    let auto = AutoLaunch::new(
        "laches_mon",
        laches_mon_path.to_str().ok_or("invalid path")?,
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
            return Err("error: invalid option for autostart. use 'yes' or 'no'.".into());
        }
    }

    Ok(())
}
