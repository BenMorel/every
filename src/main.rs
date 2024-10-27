use args::{Action, Config};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU16, Ordering};
use std::sync::Arc;
use std::time::Duration;
use std::{env, thread};

mod args;
mod help;
mod tick;

fn main() {
    let action = Action::parse(env::args());

    match action {
        Ok(Action::Help) => help::print_help(),
        Ok(Action::Version) => help::print_version(),
        Ok(Action::Run(config)) => run(config),
        Err(err) => {
            eprintln!("{err}");
            std::process::exit(1);
        }
    }
}

fn run(config: Config) -> ! {
    let interval = Duration::from_millis(config.interval_ms);

    let child_count = Arc::new(AtomicU16::new(0));
    let config = Arc::new(config);

    tick::tick(interval, || {
        let child_count = Arc::clone(&child_count);

        if child_count.load(Ordering::SeqCst) >= config.concurrency {
            return;
        }

        let config = Arc::clone(&config);

        thread::spawn(move || {
            let child = Command::new(&*config.command)
                .args(&*config.args)
                .stdin(Stdio::null())
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .spawn();

            let mut child = match child {
                Ok(child) => child,
                Err(e) => {
                    eprintln!("Failed to start command: {e}");
                    return;
                }
            };

            child_count.fetch_add(1, Ordering::SeqCst);

            match child.wait() {
                Ok(status) => {
                    if !status.success() {
                        eprintln!("Command exited with {status}");
                    }
                }
                Err(e) => {
                    // todo: we're in unsafe territory here:
                    //       we don't know if the child process is still running,
                    //       and whether we should decrement the child count;
                    //       should we panic the main thread instead?
                    eprintln!("Error checking child process status: {e}");
                }
            }

            child_count.fetch_sub(1, Ordering::SeqCst);
        });
    });
}
