use std::env;
use std::io;
use std::io::IsTerminal;

const BOLD: &str = "\x1b[1m";
const DIM: &str = "\x1b[2m";
const UNDERLINE: &str = "\x1b[4m";
const RESET: &str = "\x1b[0m";

pub fn print_help() {
    let environment = Environment::get_current();

    let (b, d, u, r) = if environment.supports_color() {
        (BOLD, DIM, UNDERLINE, RESET)
    } else {
        ("", "", "", "")
    };

    println!("Run a command every N seconds.

{u}Usage:{r}

  every -h | -v
  every <interval> [-c <n>] <command> [args...]

{u}Examples:{r}

  {d}# Run the date --utc command every second:{r}
  every 1s date --utc

  {d}# Run a curl command every 2.5 seconds,{r}
  {d}# with up to 10 commands running concurrently:{r}
  every 2.5s -c 10 curl https://...

{u}Arguments:{r}

  <interval>  The time between each command execution.
              Examples: {b}1s{r}, {b}0.75s{r}, {b}1m30s{r}, {b}1h2m3s{r}.
              Available units: {b}s{r} (seconds), {b}m{r} (minutes), {b}h{r} (hours), {b}d{r} (days).
  <command>   The command to run, followed by its arguments.

{u}Standalone Options:{r}

  -h      Show this help message and exit.
  -v      Show version information and exit.

{u}Interval Options:{r}

  -c <n>  Set the concurrency level (default: 1).
          This option must follow the interval."
    );
}

pub fn print_version() {
    println!("every {}", env!("CARGO_PKG_VERSION"));
}

struct Environment {
    is_terminal: bool,
    env_no_color: Option<String>,
    env_term: Option<String>,
}

impl Environment {
    fn get_current() -> Self {
        Self {
            is_terminal: io::stdout().is_terminal(),
            env_no_color: env::var("NO_COLOR").ok(),
            env_term: env::var("TERM").ok(),
        }
    }

    #[cfg(test)]
    fn mock(is_terminal: bool, env_vars: &Vec<(&str, &str)>) -> Self {
        let get_env_var = |key| {
            env_vars
                .iter()
                .find(|(k, _)| *k == key)
                .map(|(_, v)| v.to_string())
        };

        Self {
            is_terminal,
            env_no_color: get_env_var("NO_COLOR"),
            env_term: get_env_var("TERM"),
        }
    }

    fn supports_color(&self) -> bool {
        self.is_terminal
            // any non-empty value for NO_COLOR should disable colors
            // https://no-color.org/
            && self.env_no_color.as_ref().map_or(true, |v| v.is_empty())
            // TERM=dumb should also disable colors
            && self.env_term.as_ref().map_or(true, |v| v != "dumb")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_environment_supports_color() {
        let test_cases = [
            (false, vec![], false),
            (true, vec![], true),
            (true, vec![("NO_COLOR", "")], true),
            (true, vec![("NO_COLOR", "0")], false),
            (true, vec![("NO_COLOR", "1")], false),
            (true, vec![("TERM", "xterm")], true),
            (true, vec![("TERM", "dumb")], false),
            (true, vec![("NO_COLOR", ""), ("TERM", "dumb")], false),
            (true, vec![("NO_COLOR", "1"), ("TERM", "xterm")], false),
            (true, vec![("NO_COLOR", ""), ("TERM", "xterm")], true),
        ];

        for (is_terminal, env_vars, expected_supports_color) in test_cases {
            let environment = Environment::mock(is_terminal, &env_vars);

            assert_eq!(
                environment.supports_color(),
                expected_supports_color,
                "{:?}",
                (is_terminal, &env_vars, expected_supports_color)
            );
        }
    }
}
