use assert_cmd::prelude::*;
use nix::sys::signal::{kill, Signal};
use nix::unistd::Pid;
use std::io::BufRead;
use std::io::BufReader;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

pub fn get_cmd() -> Command {
    Command::cargo_bin("every").unwrap()
}

pub struct RunTestCase {
    pub args: Vec<&'static str>,
    pub run_time_ms: u64,
    pub grace_period_ms: u64,
    pub expected_stdout: Vec<TimestampedOutputLine>,
    pub expected_stderr: Vec<TimestampedOutputLine>,
}

pub struct TimestampedOutputLine {
    pub timestamp_ms: u64,
    pub line: String,
}

impl TimestampedOutputLine {
    pub fn repeat_at(timestamps_ms: &[u64], line: &str) -> Vec<TimestampedOutputLine> {
        timestamps_ms
            .iter()
            .map(|timestamp_ms| TimestampedOutputLine {
                timestamp_ms: *timestamp_ms,
                line: String::from(line),
            })
            .collect()
    }
}

pub fn test_run(test: RunTestCase) {
    let mut child = get_cmd()
        .args(test.args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    let start_time = Instant::now();

    let stdout = child.stdout.take().expect("Failed to capture stdout");
    let stderr = child.stderr.take().expect("Failed to capture stderr");

    // capture timestamped stdout & stderr in separate threads
    let stdout_thread_handle = thread::spawn(move || capture_output(stdout, start_time));
    let stderr_thread_handle = thread::spawn(move || capture_output(stderr, start_time));

    // let the process run for a while
    thread::sleep(Duration::from_millis(test.run_time_ms));

    // simulate ctrl-c
    let pid = Pid::from_raw(child.id().try_into().unwrap());
    kill(pid, Signal::SIGINT).expect("Failed to send SIGINT");

    // give the process some time to exit
    thread::sleep(Duration::from_millis(test.grace_period_ms));

    match child.try_wait() {
        Ok(Some(_)) => {}
        Ok(None) => {
            child
                .kill()
                .expect("Child process has not yet exited + failed to kill process!");
            panic!("Child process has not yet exited!");
        }
        Err(e) => panic!("Failed to wait for child process: {e}"),
    }

    let stdout_lines = stdout_thread_handle
        .join()
        .expect("Failed to join stdout reader thread");

    let stderr_lines = stderr_thread_handle
        .join()
        .expect("Failed to join stderr reader thread");

    assert_output_lines("stdout", &stdout_lines, &test.expected_stdout);
    assert_output_lines("stderr", &stderr_lines, &test.expected_stderr);
}

fn capture_output<R: std::io::Read>(reader: R, start_time: Instant) -> Vec<TimestampedOutputLine> {
    let reader = BufReader::new(reader);
    let mut output_lines = Vec::new();

    for line in reader.lines() {
        let line = line.unwrap();
        let timestamp_ms = start_time.elapsed().as_millis().try_into().unwrap();
        output_lines.push(TimestampedOutputLine { timestamp_ms, line });
    }

    output_lines
}

fn assert_output_lines(
    output_name: &str,
    actual_output_lines: &[TimestampedOutputLine],
    expected_output_lines: &[TimestampedOutputLine],
) {
    let fail = |message| -> ! {
        let expected = output_lines_to_string(expected_output_lines, true);
        let actual = output_lines_to_string(actual_output_lines, false);

        panic!("{output_name}: {message}\n\nExpected:\n{expected}\n\nActual:\n{actual}");
    };

    if actual_output_lines.len() != expected_output_lines.len() {
        fail(format!(
            "expected {} lines, found {}",
            expected_output_lines.len(),
            actual_output_lines.len()
        ));
    }

    for (index, (actual, expected)) in actual_output_lines
        .iter()
        .zip(expected_output_lines.iter())
        .enumerate()
    {
        let line_number = index + 1;
        let (min_timestamp_ms, max_timestamp_ms) = get_min_max_timestamp_ms(expected.timestamp_ms);

        if (actual.timestamp_ms < min_timestamp_ms) || (actual.timestamp_ms > max_timestamp_ms) {
            fail(format!(
                "line {line_number}: timestamp {} does not match the expected timestamp {}; allowed range is {}–{}",
                actual.timestamp_ms, expected.timestamp_ms, min_timestamp_ms, max_timestamp_ms
            ));
        }

        if actual.line != expected.line {
            fail(format!("line {line_number}: output does not match",));
        }
    }
}

fn output_lines_to_string(lines: &[TimestampedOutputLine], approx: bool) -> String {
    lines
        .iter()
        .map(|item| {
            format!(
                "{}{} ms: {}",
                if approx { "~" } else { "" },
                item.timestamp_ms,
                item.line
            )
        })
        .collect::<Vec<String>>()
        .join("\n")
}

const ALLOWED_JITTER_MS: u64 = 5;
const ALLOWED_STARTUP_TIME_MS: u64 = 10;

fn get_min_max_timestamp_ms(expected_timestamp_ms: u64) -> (u64, u64) {
    let min_timestamp_ms = expected_timestamp_ms.saturating_sub(ALLOWED_JITTER_MS);
    let max_timestamp_ms = expected_timestamp_ms + ALLOWED_JITTER_MS + ALLOWED_STARTUP_TIME_MS;

    (min_timestamp_ms, max_timestamp_ms)
}
