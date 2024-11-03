use std::thread;
use std::time::{Duration, Instant};

pub fn tick<F>(interval: Duration, mut function: F) -> !
where
    F: FnMut(),
{
    let mut next_tick = Instant::now() + interval;

    loop {
        function();

        let now = Instant::now();

        loop {
            if next_tick > now {
                break;
            }

            // behind schedule, skip ticks to catch up
            next_tick += interval;
        }

        thread::sleep(next_tick.duration_since(now));
    }
}
