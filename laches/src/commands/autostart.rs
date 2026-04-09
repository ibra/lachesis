use crate::cli::AutostartToggle;
use auto_launch::AutoLaunch;
use std::{error::Error, path::Path};

pub fn handle_autostart(toggle: &AutostartToggle, config_dir: &Path) -> Result<(), Box<dyn Error>> {
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
    // on windows, the auto-launch crate writes unquoted paths to the registry
    // Run key, so paths with spaces (e.g. "C:\Users\John Smith\...") break
    // silently. we quote both the exe path and args to avoid this.
    let app_path_str = laches_mon_path.to_str().ok_or("invalid path")?;

    #[cfg(windows)]
    let app_path_quoted = format!("\"{}\"", app_path_str);
    #[cfg(not(windows))]
    let app_path_quoted = app_path_str.to_string();

    #[cfg(windows)]
    let args = vec![format!("\"{}\"", config_dir.to_string_lossy())];
    #[cfg(not(windows))]
    let args = vec![config_dir.to_string_lossy().to_string()];

    #[cfg(target_os = "macos")]
    let auto = AutoLaunch::new("laches_mon", &app_path_quoted, false, &args);

    #[cfg(not(target_os = "macos"))]
    let auto = AutoLaunch::new("laches_mon", &app_path_quoted, &args);

    match toggle {
        AutostartToggle::On => {
            if auto.is_enabled()? {
                println!("info: autostart is already enabled.");
            } else {
                auto.enable()?;
                println!("info: enabled laches_mon to run at startup.");
            }
        }
        AutostartToggle::Off => {
            if !auto.is_enabled()? {
                println!("info: autostart is already disabled.");
            } else {
                auto.disable()?;
                println!("info: disabled laches_mon from running at startup.");
            }
        }
    }

    Ok(())
}
