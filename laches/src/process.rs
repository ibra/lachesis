use crate::config::{clear_daemon_pid, read_daemon_pid, write_daemon_pid};
use std::env;
use std::process::Stdio;
use std::{error::Error, path::Path, process::Command};
use sysinfo::{Pid, System};

fn is_daemon_running(pid: u32) -> bool {
    let s = System::new_all();
    if let Some(process) = s.process(Pid::from(pid as usize)) {
        let name = process.name().to_string();
        name.contains("laches_mon")
    } else {
        false
    }
}

pub fn start_monitoring(config_dir: &Path) -> Result<(), Box<dyn Error>> {
    if let Some(pid) = read_daemon_pid(config_dir) {
        if is_daemon_running(pid) {
            return Err(format!(
                "error: laches_mon is already running (pid: {}). stop it first with `laches stop`",
                pid
            )
            .into());
        }
    }

    let mut exe_path = env::current_exe()?;
    exe_path.pop();
    exe_path.push("laches_mon");

    let instance = Command::new(&exe_path)
        .arg(config_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| {
            format!(
                "error: failed to start laches_mon at '{}': {}",
                exe_path.display(),
                e
            )
        })?;

    let pid = instance.id();
    write_daemon_pid(config_dir, pid)?;
    drop(instance);

    println!("info: started laches_mon daemon (pid: {})", pid);

    Ok(())
}

pub fn stop_monitoring(config_dir: &Path) -> Result<(), Box<dyn Error>> {
    let pid = match read_daemon_pid(config_dir) {
        Some(pid) if is_daemon_running(pid) => pid,
        _ => {
            println!("info: laches_mon is not running");
            clear_daemon_pid(config_dir);
            return Ok(());
        }
    };

    let s = System::new_all();
    if let Some(process) = s.process(Pid::from(pid as usize)) {
        process.kill();
    }
    clear_daemon_pid(config_dir);
    println!("info: stopped laches_mon (pid: {})", pid);

    Ok(())
}
