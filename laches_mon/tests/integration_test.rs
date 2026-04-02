use laches::{
    config::{load_or_create_config, machine_db_path},
    db::Database,
    platform::create_tracker,
    store::get_machine_id,
};
use tempfile::TempDir;

#[test]
fn test_config_loads_in_temp_dir() {
    let temp_dir = TempDir::new().unwrap();
    let config = load_or_create_config(temp_dir.path()).unwrap();
    assert_eq!(config.daemon.check_interval, 2);
    assert_eq!(config.daemon.idle_timeout, 300);
}

#[test]
fn test_database_opens_in_data_dir() {
    let temp_dir = TempDir::new().unwrap();
    let machine_id = get_machine_id(temp_dir.path());
    let data_dir = laches::config::data_dir(temp_dir.path());
    std::fs::create_dir_all(&data_dir).unwrap();

    let db_path = machine_db_path(temp_dir.path(), &machine_id);
    let db = Database::open(&db_path).unwrap();

    let sid = db
        .start_session(
            "test_process",
            Some("/usr/bin/test"),
            Some("Test Window"),
            false,
        )
        .unwrap();
    assert!(sid > 0);

    let open = db.get_open_session().unwrap();
    assert!(open.is_some());
    assert_eq!(open.unwrap().process_name, "test_process");
}

#[test]
fn test_focus_tracker_does_not_panic() {
    let tracker = create_tracker();
    // should not panic on any platform, even without a desktop session
    let _ = tracker.get_focused_window();
    let _ = tracker.get_idle_duration();
}

#[test]
fn test_session_lifecycle() {
    let temp_dir = TempDir::new().unwrap();
    let machine_id = get_machine_id(temp_dir.path());
    let data_dir = laches::config::data_dir(temp_dir.path());
    std::fs::create_dir_all(&data_dir).unwrap();

    let db_path = machine_db_path(temp_dir.path(), &machine_id);
    let db = Database::open(&db_path).unwrap();

    // simulate daemon: start session, end it, start another
    let s1 = db
        .start_session("firefox", None, Some("GitHub"), false)
        .unwrap();
    db.end_session(s1).unwrap();

    let s2 = db
        .start_session("code", None, Some("main.rs"), false)
        .unwrap();
    db.end_session(s2).unwrap();

    let tracked = db.get_tracked_processes().unwrap();
    assert_eq!(tracked.len(), 2);
    assert!(tracked.contains(&"code".to_string()));
    assert!(tracked.contains(&"firefox".to_string()));
}

#[test]
fn test_stale_session_cleanup() {
    let temp_dir = TempDir::new().unwrap();
    let machine_id = get_machine_id(temp_dir.path());
    let data_dir = laches::config::data_dir(temp_dir.path());
    std::fs::create_dir_all(&data_dir).unwrap();

    let db_path = machine_db_path(temp_dir.path(), &machine_id);
    let db = Database::open(&db_path).unwrap();

    // simulate a crash: session left open
    db.start_session("firefox", None, None, false).unwrap();
    db.start_session("code", None, None, false).unwrap();

    let open = db.get_open_session().unwrap();
    assert!(open.is_some());

    // close all stale sessions (like daemon does on startup)
    let closed = db.close_all_open_sessions().unwrap();
    assert_eq!(closed, 2);

    let open = db.get_open_session().unwrap();
    assert!(open.is_none());
}
