use std::{
    env,
    fs::{self, File},
    io::{BufReader, Write},
    path::Path,
    thread,
    time::{Duration, Instant},
};

use laches::{process::get_active_processes, store::LachesStore};

fn tick(store_path: &Path, update_interval: &Duration) -> Result<(), std::io::Error> {
    let file = File::open(store_path)?;

    let reader = BufReader::new(&file);
    let mut r_store: LachesStore = serde_json::from_reader(reader)?;

    let store_dir = store_path.parent().unwrap_or(store_path);
    let current_machine_processes = r_store.get_machine_processes_mut(store_dir);

    for active_process in get_active_processes() {
        let mut found: bool = false;

        for stored_process in current_machine_processes.iter_mut() {
            if active_process.title == stored_process.title {
                stored_process.add_time(update_interval.as_secs());
                found = true;
                break;
            }
        }

        if !found {
            let mut new_process = active_process;
            new_process.add_time(update_interval.as_secs());
            current_machine_processes.push(new_process);
        }
    }

    let serialized_store = serde_json::to_string_pretty(&r_store)?;

    let tmp_path = store_path.with_extension("json.tmp");
    let mut tmp_file = File::create(&tmp_path)?;
    tmp_file.write_all(serialized_store.as_bytes())?;
    tmp_file.flush()?;

    fs::rename(&tmp_path, store_path)?;
    Ok(())
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 3 {
        eprintln!("usage: laches_mon <update_interval> <path/to/store>");
        return;
    }

    let update_interval: Duration = match args[1].parse() {
        Ok(num) => Duration::from_secs(num),
        Err(err) => {
            eprintln!("error: {}", err);
            eprintln!("usage: laches_mon <update_interval>");
            return;
        }
    };

    let file_path = Path::new(args[2].as_str());

    if !file_path.exists() {
        println!(
            "error: store file does not exist at location:  \"{0}\"",
            &file_path.display()
        );
        std::process::exit(1);
    }

    let mut last_tick = Instant::now();

    loop {
        let elapsed = last_tick.elapsed();
        if elapsed >= update_interval {
            tick(file_path, &update_interval)
                .expect("error: daemon failed while monitoring windows");
            last_tick = Instant::now();
        }
        thread::sleep(update_interval);
    }
}
