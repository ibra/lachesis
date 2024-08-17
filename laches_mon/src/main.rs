use std::{
    env,
    fs::File,
    io::{BufReader, Write},
    panic,
    path::Path,
    thread,
    time::{Duration, Instant},
};

use laches::{process::get_active_processes, store::LachesStore};

fn tick(store_path: &Path, update_interval: &Duration) -> Result<(), std::io::Error> {
    let file = File::open(store_path)?;

    let reader = BufReader::new(&file);
    let mut r_store: LachesStore = serde_json::from_reader(reader)?;

    for active_process in get_active_processes() {
        let mut found: bool = false;

        for stored_process in &mut r_store.process_information {
            if active_process.title == stored_process.title {
                stored_process.uptime += update_interval.as_secs();
                found = true;
                break;
            }
        }

        if !found {
            r_store.process_information.push(active_process);
        }
    }

    let serialized_store = serde_json::to_string(&r_store)?;

    let mut w_store = match File::create(store_path) {
        Err(err) => panic!("error: couldn't write to file: {}", err),
        Ok(file) => file,
    };

    w_store.write_all(serialized_store.as_bytes())?;
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
