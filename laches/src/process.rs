use crate::config::{clear_daemon_pid, read_daemon_pid, write_daemon_pid};
use crate::error::LachesError;
use std::env;
use std::process::Stdio;
use std::{path::Path, process::Command, thread, time::Duration};
use sysinfo::{Pid, ProcessRefreshKind, System, UpdateKind};

/// Check if a process with the given PID is running and is laches_mon.
/// Uses a targeted process refresh instead of scanning all processes.
fn find_daemon_process(sys: &mut System, pid: u32) -> bool {
    let sysinfo_pid = Pid::from(pid as usize);
    sys.refresh_process_specifics(
        sysinfo_pid,
        ProcessRefreshKind::new().with_cmd(UpdateKind::OnlyIfNotSet),
    );
    sys.process(sysinfo_pid)
        .map(|p| p.name().contains("laches_mon"))
        .unwrap_or(false)
}

pub fn is_daemon_running(config_dir: &Path) -> bool {
    match read_daemon_pid(config_dir) {
        Some(pid) => {
            let mut sys = System::new();
            find_daemon_process(&mut sys, pid)
        }
        None => false,
    }
}

pub fn start_monitoring(config_dir: &Path) -> Result<(), LachesError> {
    if let Some(pid) = read_daemon_pid(config_dir) {
        let mut sys = System::new();
        if find_daemon_process(&mut sys, pid) {
            return Err(format!(
                "error: laches_mon is already running (pid: {}). stop it first with `laches stop`",
                pid
            )
            .into());
        }
    }

    let mut exe_path = env::current_exe()?;
    exe_path.pop();
    if cfg!(windows) {
        exe_path.push("laches_mon.exe");
    } else {
        exe_path.push("laches_mon");
    }

    let mut child = Command::new(&exe_path)
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

    let pid = child.id();

    thread::sleep(Duration::from_millis(500));
    match child.try_wait() {
        Ok(Some(status)) => {
            return Err(format!(
                "error: laches_mon exited immediately (status: {}). check daemon.log for details",
                status
            )
            .into());
        }
        Ok(None) => {}
        Err(e) => {
            return Err(format!("error: failed to check daemon status: {}", e).into());
        }
    }

    write_daemon_pid(config_dir, pid)?;
    drop(child);

    println!("info: started laches_mon daemon (pid: {})", pid);

    Ok(())
}

/// Stop the monitoring daemon. Uses a single process lookup to avoid
/// TOCTOU races between checking and killing.
pub fn stop_monitoring(config_dir: &Path) -> Result<(), LachesError> {
    let pid = match read_daemon_pid(config_dir) {
        Some(pid) => pid,
        None => {
            println!("info: laches_mon is not running");
            return Ok(());
        }
    };

    let mut sys = System::new();
    let sysinfo_pid = Pid::from(pid as usize);
    sys.refresh_process_specifics(
        sysinfo_pid,
        ProcessRefreshKind::new().with_cmd(UpdateKind::OnlyIfNotSet),
    );

    if let Some(process) = sys.process(sysinfo_pid) {
        if process.name().contains("laches_mon") {
            process.kill();
            clear_daemon_pid(config_dir);
            println!("info: stopped laches_mon (pid: {})", pid);
        } else {
            // PID exists but isn't laches_mon — stale pid file
            clear_daemon_pid(config_dir);
            println!("info: laches_mon is not running (stale pid file)");
        }
    } else {
        // process doesn't exist at all
        clear_daemon_pid(config_dir);
        println!("info: laches_mon is not running");
    }

    Ok(())
}
