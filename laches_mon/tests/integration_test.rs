use laches::{
    process::get_active_processes,
    store::{LachesStore, Process, STORE_NAME},
};
use std::fs::File;
use std::io::Write;
use std::thread;
use std::time::Duration;
use tempfile::TempDir;

#[test]
fn test_store_exists_and_valid() {
    let temp_dir = TempDir::new().unwrap();
    let store_path = temp_dir.path();

    // Create a valid store file
    let mut store = LachesStore::default();
    store.update_interval = 5;

    let file_path = store_path.join(STORE_NAME);
    let mut file = File::create(&file_path).unwrap();
    let serialized = serde_json::to_string(&store).unwrap();
    file.write_all(serialized.as_bytes()).unwrap();

    assert!(file_path.exists());

    let file = File::open(&file_path).unwrap();
    let loaded_store: LachesStore = serde_json::from_reader(file).unwrap();
    assert_eq!(loaded_store.update_interval, 5);
}

#[test]
fn test_add_process_to_store() {
    let temp_dir = TempDir::new().unwrap();
    let store_path = temp_dir.path();

    // Create initial store
    let mut store = LachesStore::default();
    store.update_interval = 5;

    let file_path = store_path.join(STORE_NAME);
    let mut file = File::create(&file_path).unwrap();
    let serialized = serde_json::to_string(&store).unwrap();
    file.write_all(serialized.as_bytes()).unwrap();

    // Read and modify the store (simulating what tick() does)
    let file = File::open(&file_path).unwrap();
    let mut loaded_store: LachesStore = serde_json::from_reader(file).unwrap();

    let mut new_process = Process::new("test_process".to_string());
    new_process.add_time(5);
    loaded_store.process_information.push(new_process);

    // Write back
    let mut file = File::create(&file_path).unwrap();
    let serialized = serde_json::to_string(&loaded_store).unwrap();
    file.write_all(serialized.as_bytes()).unwrap();

    // Verify the process was added
    let file = File::open(&file_path).unwrap();
    let final_store: LachesStore = serde_json::from_reader(file).unwrap();
    assert_eq!(final_store.process_information.len(), 1);
    assert_eq!(final_store.process_information[0].title, "test_process");
    assert_eq!(final_store.process_information[0].uptime, 5);
}

#[test]
fn test_update_existing_process() {
    let temp_dir = TempDir::new().unwrap();
    let store_path = temp_dir.path();

    // Create store with an existing process
    let mut store = LachesStore::default();
    let mut process = Process::new("existing_process".to_string());
    process.add_time(10);
    store.process_information.push(process);

    let file_path = store_path.join(STORE_NAME);
    let mut file = File::create(&file_path).unwrap();
    let serialized = serde_json::to_string(&store).unwrap();
    file.write_all(serialized.as_bytes()).unwrap();

    // Simulate tick: read, update existing process, write
    let file = File::open(&file_path).unwrap();
    let mut loaded_store: LachesStore = serde_json::from_reader(file).unwrap();

    for stored_process in &mut loaded_store.process_information {
        if stored_process.title == "existing_process" {
            stored_process.add_time(5);
            break;
        }
    }

    let mut file = File::create(&file_path).unwrap();
    let serialized = serde_json::to_string(&loaded_store).unwrap();
    file.write_all(serialized.as_bytes()).unwrap();

    // Verify the update
    let file = File::open(&file_path).unwrap();
    let final_store: LachesStore = serde_json::from_reader(file).unwrap();
    assert_eq!(final_store.process_information.len(), 1);
    assert_eq!(final_store.process_information[0].uptime, 15);
}

#[test]
fn test_multiple_tick_cycles() {
    let temp_dir = TempDir::new().unwrap();
    let store_path = temp_dir.path();

    // Create initial store
    let mut store = LachesStore::default();
    store.update_interval = 5;

    let file_path = store_path.join(STORE_NAME);
    let mut file = File::create(&file_path).unwrap();
    let serialized = serde_json::to_string(&store).unwrap();
    file.write_all(serialized.as_bytes()).unwrap();

    // Simulate multiple tick cycles
    for _cycle in 1..=3 {
        let file = File::open(&file_path).unwrap();
        let mut loaded_store: LachesStore = serde_json::from_reader(file).unwrap();

        // Add or update a process
        let mut found = false;
        for stored_process in &mut loaded_store.process_information {
            if stored_process.title == "test_process" {
                stored_process.add_time(store.update_interval);
                found = true;
                break;
            }
        }

        if !found {
            let mut new_process = Process::new("test_process".to_string());
            new_process.add_time(store.update_interval);
            loaded_store.process_information.push(new_process);
        }

        let mut file = File::create(&file_path).unwrap();
        let serialized = serde_json::to_string(&loaded_store).unwrap();
        file.write_all(serialized.as_bytes()).unwrap();
    }

    let file = File::open(&file_path).unwrap();
    let final_store: LachesStore = serde_json::from_reader(file).unwrap();
    assert_eq!(final_store.process_information.len(), 1);
    assert_eq!(final_store.process_information[0].title, "test_process");
    assert_eq!(final_store.process_information[0].uptime, 15); // 3 cycles * 5 seconds
}

#[test]
fn test_get_active_processes_integration() {
    let processes = get_active_processes();

    for process in &processes {
        assert!(!process.title.is_empty());
        assert_eq!(process.uptime, 0); // new processes start with 0 uptime
    }
}

#[test]
fn test_concurrent_process_tracking() {
    let temp_dir = TempDir::new().unwrap();
    let store_path = temp_dir.path();

    // Create initial store
    let mut store = LachesStore::default();
    store.update_interval = 5;

    let file_path = store_path.join(STORE_NAME);
    let mut file = File::create(&file_path).unwrap();
    let serialized = serde_json::to_string(&store).unwrap();
    file.write_all(serialized.as_bytes()).unwrap();

    // Simulate tracking multiple processes
    let processes_to_track = vec!["process1", "process2", "process3"];

    for process_name in &processes_to_track {
        let file = File::open(&file_path).unwrap();
        let mut loaded_store: LachesStore = serde_json::from_reader(file).unwrap();

        let mut new_process = Process::new(process_name.to_string());
        new_process.add_time(5);
        loaded_store.process_information.push(new_process);

        let mut file = File::create(&file_path).unwrap();
        let serialized = serde_json::to_string(&loaded_store).unwrap();
        file.write_all(serialized.as_bytes()).unwrap();
    }

    // Verify all processes were tracked
    let file = File::open(&file_path).unwrap();
    let final_store: LachesStore = serde_json::from_reader(file).unwrap();
    assert_eq!(final_store.process_information.len(), 3);

    let titles: Vec<String> = final_store
        .process_information
        .iter()
        .map(|p| p.title.clone())
        .collect();

    for process_name in &processes_to_track {
        assert!(titles.contains(&process_name.to_string()));
    }
}

#[test]
fn test_store_persistence() {
    let temp_dir = TempDir::new().unwrap();
    let store_path = temp_dir.path();

    // Create and save store
    let mut store = LachesStore::default();
    let mut process = Process::new("persistent_process".to_string());
    process.add_time(100);
    store.process_information.push(process);

    let file_path = store_path.join(STORE_NAME);
    let mut file = File::create(&file_path).unwrap();
    let serialized = serde_json::to_string(&store).unwrap();
    file.write_all(serialized.as_bytes()).unwrap();

    thread::sleep(Duration::from_millis(10));

    let file = File::open(&file_path).unwrap();
    let loaded_store: LachesStore = serde_json::from_reader(file).unwrap();

    assert_eq!(loaded_store.process_information.len(), 1);
    assert_eq!(
        loaded_store.process_information[0].title,
        "persistent_process"
    );
    assert_eq!(loaded_store.process_information[0].uptime, 100);
}
