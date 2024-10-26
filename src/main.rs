use args::{Action, Config};
use std::env;
use std::process::{Child, Command, Stdio};
use std::time::Duration;

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
    let mut children: Vec<Child> = Vec::new();

    tick::tick(interval, || {
        children.retain_mut(|child| match child.try_wait() {
            Ok(Some(status)) => {
                if !status.success() {
                    eprintln!("Command exited with {status}");
                }

                false
            }
            Ok(None) => true,
            Err(e) => {
                eprintln!("Error checking child process status: {e}");
                true
            }
        });

        if children.len() < config.concurrency as usize {
            let child = Command::new(&config.command)
                .args(&config.args)
                .stdin(Stdio::null())
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .spawn();

            match child {
                Ok(child) => {
                    children.push(child);
                }
                Err(e) => {
                    eprintln!("Failed to start command: {e}");
                }
            }
        }
    });
}
