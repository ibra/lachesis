use laches::{
    process::get_active_processes,
    store::{get_machine_id, LachesStore, Process, STORE_NAME},
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

    let mut store = LachesStore::default();
    store.update_interval = 5;

    let file_path = store_path.join(STORE_NAME);
    let mut file = File::create(&file_path).unwrap();
    let serialized = serde_json::to_string(&store).unwrap();
    file.write_all(serialized.as_bytes()).unwrap();

    let file = File::open(&file_path).unwrap();
    let mut loaded_store: LachesStore = serde_json::from_reader(file).unwrap();

    let mut new_process = Process::new("test_process".to_string());
    new_process.add_time(5);
    let current_machine_processes = loaded_store.get_machine_processes_mut(store_path);
    current_machine_processes.push(new_process);

    let mut file = File::create(&file_path).unwrap();
    let serialized = serde_json::to_string(&loaded_store).unwrap();
    file.write_all(serialized.as_bytes()).unwrap();

    let file = File::open(&file_path).unwrap();
    let final_store: LachesStore = serde_json::from_reader(file).unwrap();
    let machine_id = get_machine_id(store_path);
    let processes = final_store.machine_data.get(&machine_id).unwrap();
    assert_eq!(processes.len(), 1);
    assert_eq!(processes[0].title, "test_process");
    assert_eq!(processes[0].get_total_usage(), 5);
}

#[test]
fn test_update_existing_process() {
    let temp_dir = TempDir::new().unwrap();
    let store_path = temp_dir.path();

    let mut store = LachesStore::default();
    let machine_id = get_machine_id(store_path);
    let mut process = Process::new("existing_process".to_string());
    process.add_time(10);
    store.machine_data.insert(machine_id.clone(), vec![process]);

    let file_path = store_path.join(STORE_NAME);
    let mut file = File::create(&file_path).unwrap();
    let serialized = serde_json::to_string(&store).unwrap();
    file.write_all(serialized.as_bytes()).unwrap();

    let file = File::open(&file_path).unwrap();
    let mut loaded_store: LachesStore = serde_json::from_reader(file).unwrap();

    let current_machine_processes = loaded_store.get_machine_processes_mut(store_path);
    for stored_process in current_machine_processes.iter_mut() {
        if stored_process.title == "existing_process" {
            stored_process.add_time(5);
            break;
        }
    }

    let mut file = File::create(&file_path).unwrap();
    let serialized = serde_json::to_string(&loaded_store).unwrap();
    file.write_all(serialized.as_bytes()).unwrap();

    let file = File::open(&file_path).unwrap();
    let final_store: LachesStore = serde_json::from_reader(file).unwrap();
    let machine_id = get_machine_id(store_path);
    let processes = final_store.machine_data.get(&machine_id).unwrap();
    assert_eq!(processes.len(), 1);
    assert_eq!(processes[0].get_total_usage(), 15);
}

#[test]
fn test_multiple_tick_cycles() {
    let temp_dir = TempDir::new().unwrap();
    let store_path = temp_dir.path();

    let mut store = LachesStore::default();
    store.update_interval = 5;

    let file_path = store_path.join(STORE_NAME);
    let mut file = File::create(&file_path).unwrap();
    let serialized = serde_json::to_string(&store).unwrap();
    file.write_all(serialized.as_bytes()).unwrap();

    for _cycle in 1..=3 {
        let file = File::open(&file_path).unwrap();
        let mut loaded_store: LachesStore = serde_json::from_reader(file).unwrap();

        let current_machine_processes = loaded_store.get_machine_processes_mut(store_path);
        let mut found = false;
        for stored_process in current_machine_processes.iter_mut() {
            if stored_process.title == "test_process" {
                stored_process.add_time(store.update_interval);
                found = true;
                break;
            }
        }

        if !found {
            let mut new_process = Process::new("test_process".to_string());
            new_process.add_time(store.update_interval);
            current_machine_processes.push(new_process);
        }

        let mut file = File::create(&file_path).unwrap();
        let serialized = serde_json::to_string(&loaded_store).unwrap();
        file.write_all(serialized.as_bytes()).unwrap();
    }

    let file = File::open(&file_path).unwrap();
    let final_store: LachesStore = serde_json::from_reader(file).unwrap();
    let machine_id = get_machine_id(store_path);
    let processes = final_store.machine_data.get(&machine_id).unwrap();
    assert_eq!(processes.len(), 1);
    assert_eq!(processes[0].title, "test_process");
    assert_eq!(processes[0].get_total_usage(), 15); // 3 cycles * 5 seconds
}

#[test]
fn test_get_active_processes_integration() {
    let processes = get_active_processes();

    for process in &processes {
        assert!(!process.title.is_empty());
        assert_eq!(process.get_total_usage(), 0); // new processes start with 0 uptime
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

    let processes_to_track = vec!["process1", "process2", "process3"];

    for process_name in &processes_to_track {
        let file = File::open(&file_path).unwrap();
        let mut loaded_store: LachesStore = serde_json::from_reader(file).unwrap();

        let mut new_process = Process::new(process_name.to_string());
        new_process.add_time(5);
        let current_machine_processes = loaded_store.get_machine_processes_mut(store_path);
        current_machine_processes.push(new_process);

        let mut file = File::create(&file_path).unwrap();
        let serialized = serde_json::to_string(&loaded_store).unwrap();
        file.write_all(serialized.as_bytes()).unwrap();
    }

    let file = File::open(&file_path).unwrap();
    let final_store: LachesStore = serde_json::from_reader(file).unwrap();
    let machine_id = get_machine_id(store_path);
    let processes = final_store.machine_data.get(&machine_id).unwrap();
    assert_eq!(processes.len(), 3);

    let titles: Vec<String> = processes.iter().map(|p| p.title.clone()).collect();

    for process_name in &processes_to_track {
        assert!(titles.contains(&process_name.to_string()));
    }
}

#[test]
fn test_store_persistence() {
    let temp_dir = TempDir::new().unwrap();
    let store_path = temp_dir.path();

    let mut store = LachesStore::default();
    let machine_id = get_machine_id(store_path);
    let mut process = Process::new("persistent_process".to_string());
    process.add_time(100);
    store.machine_data.insert(machine_id.clone(), vec![process]);

    let file_path = store_path.join(STORE_NAME);
    let mut file = File::create(&file_path).unwrap();
    let serialized = serde_json::to_string(&store).unwrap();
    file.write_all(serialized.as_bytes()).unwrap();

    thread::sleep(Duration::from_millis(10));

    let file = File::open(&file_path).unwrap();
    let loaded_store: LachesStore = serde_json::from_reader(file).unwrap();

    let machine_id = get_machine_id(store_path);
    let processes = loaded_store.machine_data.get(&machine_id).unwrap();
    assert_eq!(processes.len(), 1);
    assert_eq!(processes[0].title, "persistent_process");
    assert_eq!(processes[0].get_total_usage(), 100);
}
