use assert_cmd::prelude::*;
use helpers::{get_cmd, test_run, RunTestCase, TimestampedOutputLine};
use predicates::prelude::*;
use std::process::Command;

mod helpers;

#[test]
fn test_help() {
    #[rustfmt::skip]
    let test_cases: [fn(&mut Command) -> &mut Command; 2] = [
        |cmd| cmd,
        |cmd| cmd.arg("-h"),
    ];

    for configure_cmd in test_cases {
        let mut cmd = get_cmd();
        configure_cmd(&mut cmd)
            .assert()
            .success()
            .stdout(
                predicates::str::contains("Run a command every N seconds.\n")
                    .and(predicates::str::contains("\nUsage:\n"))
                    .and(predicates::str::contains("\nExamples:\n"))
                    .and(predicates::str::contains("\nArguments:\n"))
                    .and(predicates::str::contains("\nStandalone Options:\n"))
                    .and(predicates::str::contains("\nInterval Options:\n")),
            )
            .stderr("");
    }
}

#[test]
fn test_version() {
    get_cmd()
        .arg("-v")
        .assert()
        .success()
        .stdout("every 0.1.0\n")
        .stderr("");
}

#[test]
fn test_invalid_option() {
    get_cmd()
        .arg("-x")
        .assert()
        .failure()
        .stdout("")
        .stderr("Invalid option: -x\n");
}

#[test]
fn test_invalid_interval() {
    get_cmd()
        .arg("2j")
        .assert()
        .failure()
        .stdout("")
        .stderr("Invalid interval '2j': unrecognized format\n");
}

#[test]
fn test_invalid_concurrency() {
    get_cmd()
        .arg("1s")
        .arg("-c")
        .arg("0")
        .assert()
        .failure()
        .stdout("")
        .stderr("Invalid concurrency: value 0 is not in the range 1–1000\n");
}

#[test]
fn test_run_echo() {
    test_run(RunTestCase {
        args: vec!["0.1s", "echo", "hello", "world"],
        run_time_ms: 550,
        grace_period_ms: 40,
        expected_stdout: TimestampedOutputLine::repeat_at(
            &[0, 100, 200, 300, 400, 500],
            "hello world",
        ),
        expected_stderr: vec![],
    });
}

#[test]
fn test_run_with_long_running_command() {
    test_run(RunTestCase {
        args: vec!["0.1s", "bash", "-c", "echo hello world && sleep 0.15"],
        run_time_ms: 650,
        grace_period_ms: 40,
        expected_stdout: TimestampedOutputLine::repeat_at(
            // skipped ticks!
            &[0, 200, 400, 600],
            "hello world",
        ),
        expected_stderr: vec![],
    });
}

#[test]
fn test_run_with_non_zero_exit_code() {
    test_run(RunTestCase {
        args: vec!["0.1s", "bash", "-c", "echo hello && false"],
        run_time_ms: 550,
        grace_period_ms: 40,
        expected_stdout: TimestampedOutputLine::repeat_at(&[0, 100, 200, 300, 400, 500], "hello"),
        expected_stderr: TimestampedOutputLine::repeat_at(
            &[0, 100, 200, 300, 400, 500],
            "Command exited with exit status: 1",
        ),
    });
}

#[test]
fn test_run_with_non_existing_command() {
    test_run(RunTestCase {
        args: vec!["0.1s", "non-existing-command", "arg1", "arg2"],
        run_time_ms: 350,
        grace_period_ms: 40,
        expected_stdout: vec![],
        expected_stderr: TimestampedOutputLine::repeat_at(
            &[0, 100, 200, 300],
            "Failed to start command: No such file or directory (os error 2)",
        ),
    });
}

#[test]
fn test_run_with_concurrency() {
    test_run(RunTestCase {
        args: vec!["0.1s", "-c", "3", "echo", "hello", "world"],
        run_time_ms: 950,
        grace_period_ms: 40,
        expected_stdout: TimestampedOutputLine::repeat_at(
            // no skipped ticks
            &[0, 100, 200, 300, 400, 500, 600, 700, 800, 900],
            "hello world",
        ),
        expected_stderr: vec![],
    });
}

#[test]
fn test_run_with_concurrency_and_long_running_command() {
    test_run(RunTestCase {
        args: vec![
            "0.1s",
            "-c",
            "3",
            "bash",
            "-c",
            "echo hello world && sleep 0.45",
        ],
        run_time_ms: 1250,
        grace_period_ms: 40,
        expected_stdout: TimestampedOutputLine::repeat_at(
            // skipped ticks!
            &[0, 100, 200, 500, 600, 700, 1000, 1100, 1200],
            "hello world",
        ),
        expected_stderr: vec![],
    });
}
