use laches::{
    config::load_or_create_config,
    db::Database,
    platform::{create_tracker, FocusInfo},
    store::get_machine_id,
};
use std::{
    env,
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::Duration,
};

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

    // load config once at startup
    let config = match load_or_create_config(config_dir) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error: failed to load config: {}", e);
            std::process::exit(1);
        }
    };

    // open the per-machine database
    let machine_id = get_machine_id(config_dir);
    let data_dir = laches::config::data_dir(config_dir);
    if let Err(e) = std::fs::create_dir_all(&data_dir) {
        eprintln!("error: failed to create data directory: {}", e);
        std::process::exit(1);
    }

    let db_path = laches::config::machine_db_path(config_dir, &machine_id);
    let db = match Database::open(&db_path) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("error: failed to open database: {}", e);
            std::process::exit(1);
        }
    };

    // close any sessions left open from a previous crash
    if let Ok(count) = db.close_all_open_sessions() {
        if count > 0 {
            eprintln!("info: closed {} stale sessions from previous run", count);
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
                    eprintln!("warning: failed to end session: {}", e);
                }
            }

            // start a new session if there's a focused window
            if is_idle {
                // record idle session
                match db.start_session("idle", None, None, true) {
                    Ok(sid) => current_session_id = Some(sid),
                    Err(e) => eprintln!("warning: failed to start idle session: {}", e),
                }
            } else if let Some(ref info) = focused {
                match db.start_session(
                    &info.process_name,
                    info.exe_path.as_deref(),
                    info.window_title.as_deref(),
                    false,
                ) {
                    Ok(sid) => current_session_id = Some(sid),
                    Err(e) => eprintln!("warning: failed to start session: {}", e),
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
            eprintln!("warning: failed to end session on shutdown: {}", e);
        }
    }

    eprintln!("info: laches_mon stopped cleanly");
}
