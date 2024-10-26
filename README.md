# ⏱️ every

Run a command every *n* seconds.

## Installation

1. Ensure you have [Cargo and Rust](https://doc.rust-lang.org/cargo/getting-started/installation.html) installed.
2. Install with the following command:

```bash
cargo install every
```

## Usage

To run `echo hello world` every second, use:

```bash
every 1s echo hello world
```

If the command takes longer than the specified interval (`1s` in this case), some ticks will be skipped because the concurrency is set to `1` by default.

To allow multiple instances of the command to run in parallel, and reduce skipped ticks, use the `-c` option:

```bash
every 1s -c 10 curl https://some-slow-api.com/
```

With this setting, up to `10` commands can run in parallel. The command will execute every second without skipped ticks, as long as the number of parallel executions doesn’t exceed the concurrency limit.

## Interval format

The interval format is a number followed by a unit. The unit can be one of the following:

- `s` for seconds
- `m` for minutes
- `h` for hours
- `d` for days

Seconds can have a decimal part: `2.5s`.  
Units can be combined: `1h30m`.