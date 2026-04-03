use laches::{
    commands::filtering::matches_any_pattern,
    config::{get_machine_id, load_or_create_config, FilterMode},
    db::Database,
    platform::{create_tracker, FocusInfo},
};
use std::{
    env,
    fs::OpenOptions,
    io::Write,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::Duration,
};

/// Simple file-based logger for the daemon.
/// Logs are written to daemon.log in the config directory.
struct DaemonLogger {
    file: std::fs::File,
}

impl DaemonLogger {
    fn open(config_dir: &Path) -> Option<Self> {
        let log_path = config_dir.join("daemon.log");
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_path)
            .ok()
            .map(|file| DaemonLogger { file })
    }

    fn log(&mut self, msg: &str) {
        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
        let _ = writeln!(self.file, "[{}] {}", timestamp, msg);
    }
}

/// Check if a process should be tracked based on the current filtering config.
fn should_track(
    process_name: &str,
    mode: &FilterMode,
    whitelist: &[laches::config::FilterPattern],
    blacklist: &[laches::config::FilterPattern],
) -> bool {
    match mode {
        FilterMode::Default => true,
        FilterMode::Whitelist => matches_any_pattern(process_name, whitelist),
        FilterMode::Blacklist => !matches_any_pattern(process_name, blacklist),
    }
}

fn init_daemon(config_dir: &Path) -> (Database, laches::config::Config, DaemonLogger, PathBuf) {
    let mut logger = DaemonLogger::open(config_dir).expect("error: failed to open daemon.log");

    let config = match load_or_create_config(config_dir) {
        Ok(c) => c,
        Err(e) => {
            logger.log(&format!("error: failed to load config: {}", e));
            std::process::exit(1);
        }
    };

    let machine_id = get_machine_id(config_dir);
    let data_dir = laches::config::data_dir(config_dir);
    if let Err(e) = std::fs::create_dir_all(&data_dir) {
        logger.log(&format!("error: failed to create data directory: {}", e));
        std::process::exit(1);
    }

    let db_path = laches::config::machine_db_path(config_dir, &machine_id);
    let db = match Database::open(&db_path) {
        Ok(d) => d,
        Err(e) => {
            logger.log(&format!("error: failed to open database: {}", e));
            std::process::exit(1);
        }
    };

    (db, config, logger, db_path)
}

/// Core monitoring loop. Extracted from main for testability.
fn run_monitor(
    db: &Database,
    config: &laches::config::Config,
    logger: &mut DaemonLogger,
    tracker: &dyn laches::platform::FocusTracker,
    running: &AtomicBool,
) {
    let check_interval = Duration::from_secs(config.daemon.check_interval);
    let idle_timeout = Duration::from_secs(config.daemon.idle_timeout);

    let mut last_focus: Option<FocusInfo> = None;
    let mut was_idle = false;
    let mut current_session_id: Option<i64> = None;

    while running.load(Ordering::SeqCst) {
        let focused = tracker.get_focused_window();
        let idle_duration = tracker.get_idle_duration();
        let is_idle = idle_duration >= idle_timeout;

        let focus_changed = focused != last_focus;
        let idle_changed = is_idle != was_idle;

        if focus_changed || idle_changed {
            // end the current session if there is one
            if let Some(sid) = current_session_id.take() {
                if let Err(e) = db.end_session(sid) {
                    logger.log(&format!("warning: failed to end session: {}", e));
                }
            }

            // start a new session based on current state
            if is_idle {
                match db.start_session("idle", None, None, true) {
                    Ok(sid) => current_session_id = Some(sid),
                    Err(e) => logger.log(&format!("warning: failed to start idle session: {}", e)),
                }
            } else if let Some(ref info) = focused {
                // apply filtering before recording
                if should_track(
                    &info.process_name,
                    &config.filtering.mode,
                    &config.filtering.whitelist,
                    &config.filtering.blacklist,
                ) {
                    match db.start_session(
                        &info.process_name,
                        info.exe_path.as_deref(),
                        info.window_title.as_deref(),
                        false,
                    ) {
                        Ok(sid) => current_session_id = Some(sid),
                        Err(e) => logger.log(&format!("warning: failed to start session: {}", e)),
                    }
                }
            }

            last_focus = focused;
            was_idle = is_idle;
        }

        thread::sleep(check_interval);
    }

    // clean shutdown: end the current session
    if let Some(sid) = current_session_id {
        if let Err(e) = db.end_session(sid) {
            logger.log(&format!(
                "warning: failed to end session on shutdown: {}",
                e
            ));
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        eprintln!("usage: laches_mon <config_dir>");
        std::process::exit(1);
    }

    let config_dir = Path::new(&args[1]);

    if !config_dir.exists() {
        eprintln!(
            "error: config directory does not exist: {}",
            config_dir.display()
        );
        std::process::exit(1);
    }

    let (db, config, mut logger, _db_path) = init_daemon(config_dir);

    // close any sessions left open from a previous crash
    if let Ok(count) = db.close_all_open_sessions() {
        if count > 0 {
            logger.log(&format!(
                "closed {} stale sessions from previous run",
                count
            ));
        }
    }

    // set up signal handler for clean shutdown
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })
    .expect("error: failed to set signal handler");

    let tracker = create_tracker();

    logger.log(&format!(
        "started (interval={}s, idle_timeout={}s, filter={})",
        config.daemon.check_interval, config.daemon.idle_timeout, config.filtering.mode
    ));

    run_monitor(&db, &config, &mut logger, tracker.as_ref(), &running);

    logger.log("stopped cleanly");
}

#[cfg(test)]
mod tests {
    use super::*;
    use laches::config::{FilterMode, FilterPattern};

    #[test]
    fn test_should_track_default_mode() {
        let wl = vec![];
        let bl = vec![];
        assert!(should_track("anything", &FilterMode::Default, &wl, &bl));
    }

    #[test]
    fn test_should_track_whitelist_mode() {
        let wl = vec![FilterPattern::exact("firefox")];
        let bl = vec![];
        assert!(should_track("firefox", &FilterMode::Whitelist, &wl, &bl));
        assert!(!should_track("chrome", &FilterMode::Whitelist, &wl, &bl));
    }

    #[test]
    fn test_should_track_blacklist_mode() {
        let wl = vec![];
        let bl = vec![FilterPattern::exact("discord")];
        assert!(!should_track("discord", &FilterMode::Blacklist, &wl, &bl));
        assert!(should_track("firefox", &FilterMode::Blacklist, &wl, &bl));
    }

    #[test]
    fn test_should_track_regex_patterns() {
        let wl = vec![FilterPattern::regex("^(firefox|chrome)$")];
        let bl = vec![];
        assert!(should_track("firefox", &FilterMode::Whitelist, &wl, &bl));
        assert!(should_track("chrome", &FilterMode::Whitelist, &wl, &bl));
        assert!(!should_track("discord", &FilterMode::Whitelist, &wl, &bl));
    }
}
