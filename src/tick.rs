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

        if next_tick > now {
            let sleep_duration = next_tick.duration_since(now);
            thread::sleep(sleep_duration);

            next_tick = next_tick + interval;
        } else {
            // we were not fast enough, reschedule the next tick from now
            next_tick = now + interval;
        }
    }
}
