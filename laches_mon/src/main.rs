use std::{env, thread, time::{Duration, Instant}};

fn tick() {
  println!("monitoring...");
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
      eprintln!("usage: laches_mon <update_interval>");
      return;
    }
  
    let value: u64 = match args[1].parse() {
      Ok(num) => num,
      Err(_) => {
        eprintln!("error: invalid argument provided");
        return;
      }
    };

  let interval = Duration::from_millis(value); 
  let mut last_tick = Instant::now();

  loop {
    let elapsed = last_tick.elapsed();
    if elapsed >= interval {
      tick();
      last_tick = Instant::now();
    }
    thread::sleep(Duration::from_millis(1)); 
  }
}