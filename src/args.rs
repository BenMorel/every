use regex::{Match, Regex};
use std::env::Args;
use std::num::IntErrorKind;

const MAX_CONCURRENCY: u16 = 1000;

#[derive(Debug, PartialEq)]
pub enum Action {
    Run(Config),
    Help,
    Version,
}

#[derive(Debug, PartialEq)]
pub struct Config {
    pub interval_ms: u64,
    pub concurrency: u16,
    pub command: String,
    pub args: Vec<String>,
}

impl Action {
    pub fn parse(args: Args) -> Result<Action, String> {
        Self::parse_iter(args.skip(1))
    }

    fn parse_iter<T>(mut args: T) -> Result<Action, String>
    where
        T: Iterator<Item = String>,
    {
        let arg = match args.next() {
            Some(arg) => arg,
            None => return Ok(Action::Help),
        };

        if arg.starts_with("-") {
            if arg == "-h" {
                return Ok(Action::Help);
            }

            if arg == "-v" {
                return Ok(Action::Version);
            }

            return Err(format!("Invalid option: {arg}"));
        }

        let interval_ms = parse_interval_as_ms(&arg)?;

        let arg = match args.next() {
            Some(arg) => arg,
            None => return Err(String::from("Missing command name!")),
        };

        let mut concurrency = 1;
        let command;

        if arg.starts_with("-") {
            if arg == "-c" {
                concurrency = match args.next() {
                    Some(arg) => parse_concurrency(&arg)?,
                    None => return Err(String::from("Missing concurrency value!")),
                };
            } else {
                return Err(format!("Invalid option after interval: {arg}"));
            }

            command = match args.next() {
                Some(arg) => arg,
                None => return Err(String::from("Missing command name!")),
            };
        } else {
            command = arg;
        }

        let args = args.collect();

        Ok(Action::Run(Config {
            interval_ms,
            concurrency,
            command,
            args,
        }))
    }
}

fn parse_interval_as_ms(interval: &str) -> Result<u64, String> {
    if interval.is_empty() {
        return Err(String::from("Interval cannot be empty"));
    }

    let re = Regex::new(concat!(
        "^",
        "(?:([0-9]+)d)?",
        "(?:([0-9]+)h)?",
        "(?:([0-9]+)m)?",
        "(?:([0-9]+)(?:\\.([0-9]+))?s)?",
        "$"
    ))
    .unwrap();

    if let Some(caps) = re.captures(interval) {
        let d = caps.get(1);
        let h = caps.get(2);
        let m = caps.get(3);
        let s = caps.get(4);
        let f = caps.get(5);

        let d = convert_match_to_u64(d);
        let h = convert_match_to_u64(h);
        let m = convert_match_to_u64(m);
        let s = convert_match_to_u64(s);

        let ms = convert_fraction_match_to_ms_u64(f).ok_or_else(|| {
            format!("Invalid interval '{interval}': maximum precision is millisecond")
        })?;

        let total_ms = calculate_total_ms(d, h, m, s, ms);

        match total_ms {
            Some(0) => Err(format!(
                "Invalid interval '{interval}': interval cannot be zero"
            )),
            Some(ms) => Ok(ms),
            None => Err(format!(
                "Invalid interval '{interval}': interval is too large"
            )),
        }
    } else {
        Err(format!(
            "Invalid interval '{interval}': unrecognized format"
        ))
    }
}

// Optional arguments, and return value, are None iff the number is too large.
fn calculate_total_ms(
    d: Option<u64>,
    h: Option<u64>,
    m: Option<u64>,
    s: Option<u64>,
    ms: u64,
) -> Option<u64> {
    d?.checked_mul(86_400_000)?
        .checked_add(h?.checked_mul(3_600_000)?)?
        .checked_add(m?.checked_mul(60_000)?)?
        .checked_add(s?.checked_mul(1_000)?)?
        .checked_add(ms)
}

// Used with [0-9]+ matches.
// Returns 0 if there is no match, and None if the number does not fit in u64.
fn convert_match_to_u64(m: Option<Match>) -> Option<u64> {
    match m {
        Some(m) => m.as_str().parse().ok(),
        None => Some(0),
    }
}

// Used with [0-9]+ matches.
// Returns 0 if there is no match, and None if the fraction is > 3 digits.
// Examples:
//  - "1" => 100,
//  - "12" => 120
//  - "123" => 123
//  - "1234" => None
fn convert_fraction_match_to_ms_u64(f: Option<Match>) -> Option<u64> {
    match f {
        Some(f) => {
            let f = f.as_str();

            match f.len() {
                len if len <= 3 => format!("{f:0<3}").parse().ok(),
                _ => None,
            }
        }
        None => Some(0),
    }
}

fn parse_concurrency(concurrency: &str) -> Result<u16, String> {
    let invalid_range = || {
        format!("Invalid concurrency: value {concurrency} is not in the range 1–{MAX_CONCURRENCY}")
    };

    match concurrency.parse() {
        Ok(concurrency) if (1..=MAX_CONCURRENCY).contains(&concurrency) => Ok(concurrency),
        Ok(_) => Err(invalid_range()),
        Err(err) if *err.kind() == IntErrorKind::PosOverflow => Err(invalid_range()),
        Err(_) => Err(format!("Invalid concurrency value: '{concurrency}'")),
    }
}

#[cfg(test)]
#[rustfmt::skip]
mod tests {
    use super::*;

    #[test]
    fn test_parse_args() {
        let test_cases = [
            // empty
            (vec![], Ok(Action::Help)),
            // help
            (vec!["-h"], Ok(Action::Help)),
            // version
            (vec!["-v"], Ok(Action::Version)),
            // invalid option
            (vec!["-x"], Err("Invalid option: -x")),
            // empty interval
            (vec![""], Err("Interval cannot be empty")),
            // invalid interval
            (vec!["1"], Err("Invalid interval '1': unrecognized format")),
            // missing command
            (vec!["1s"], Err("Missing command name!")),
            // invalid option after interval
            (vec!["1s", "-x"], Err("Invalid option after interval: -x")),
            // missing concurrency value
            (vec!["1s", "-c"], Err("Missing concurrency value!")),
            // invalid concurrency value
            (vec!["1s", "-c", "x"], Err("Invalid concurrency value: 'x'")),
            // missing command after options
            (vec!["1s", "-c", "1"], Err("Missing command name!")),
            // valid
            (vec!["1s", "date"], Ok(Action::Run(Config {
                interval_ms: 1_000,
                concurrency: 1,
                command: String::from("date"),
                args: vec![],
            }))),
            // valid with concurrency and args
            (vec!["1m5.5s", "-c", "10", "echo", "hello", "world"], Ok(Action::Run(Config {
                interval_ms: 65500,
                concurrency: 10,
                command: String::from("echo"),
                args: vec![
                    String::from("hello"),
                    String::from("world"),
                ],
            }))),
        ];

        for (args, expected) in test_cases {
            let expected = expected.map_err(|e| e.to_string());
            let args = args.into_iter().map(|s| s.to_string());
            let actual = Action::parse_iter(args.clone());

            assert_eq!(actual, expected, "args: {:?}", args.collect::<Vec<_>>());
        }
    }

    #[test]
    fn test_parse_interval_as_ms() {
        let tests_cases = [
            // empty
            ("", Err("Interval cannot be empty")),
            // unparsable
            (" ", Err("Invalid interval ' ': unrecognized format")),
            ("d", Err("Invalid interval 'd': unrecognized format")),
            ("h", Err("Invalid interval 'h': unrecognized format")),
            ("m", Err("Invalid interval 'm': unrecognized format")),
            ("s", Err("Invalid interval 's': unrecognized format")),
            (".s", Err("Invalid interval '.s': unrecognized format")),
            (".1s", Err("Invalid interval '.1s': unrecognized format")),
            ("1m.0s", Err("Invalid interval '1m.0s': unrecognized format")),
            ("0.1d", Err("Invalid interval '0.1d': unrecognized format")),
            ("0.1h", Err("Invalid interval '0.1h': unrecognized format")),
            ("0.1m", Err("Invalid interval '0.1m': unrecognized format")),
            ("-1s", Err("Invalid interval '-1s': unrecognized format")),
            (" 1s", Err("Invalid interval ' 1s': unrecognized format")),
            ("1s-", Err("Invalid interval '1s-': unrecognized format")),
            ("1s ", Err("Invalid interval '1s ': unrecognized format")),
            ("1s 1s", Err("Invalid interval '1s 1s': unrecognized format")),
            ("1.1.1s", Err("Invalid interval '1.1.1s': unrecognized format")),
            // seconds
            ("0s", Err("Invalid interval '0s': interval cannot be zero")),
            ("0.0s", Err("Invalid interval '0.0s': interval cannot be zero")),
            ("0.00s", Err("Invalid interval '0.00s': interval cannot be zero")),
            ("0.000s", Err("Invalid interval '0.000s': interval cannot be zero")),
            ("0.0000s", Err("Invalid interval '0.0000s': maximum precision is millisecond")),
            ("1s", Ok(1_000)),
            ("23s", Ok(23_000)),
            ("0.1s", Ok(100)),
            ("0.01s", Ok(10)),
            ("0.12s", Ok(120)),
            ("0.001s", Ok(1)),
            ("0.012s", Ok(12)),
            ("0.123s", Ok(123)),
            ("0.1234s", Err("Invalid interval '0.1234s': maximum precision is millisecond")),
            ("1234.5s", Ok(1234_500)),
            ("1234.56s", Ok(1234_560)),
            ("1234.567s", Ok(1234_567)),
            ("1234.5678s", Err("Invalid interval '1234.5678s': maximum precision is millisecond")),
            // minutes
            ("0m", Err("Invalid interval '0m': interval cannot be zero")),
            ("1m", Ok(60 * 1000)),
            ("123m", Ok(123 * 60 * 1000)),
            // minutes + seconds
            ("0m0s", Err("Invalid interval '0m0s': interval cannot be zero")),
            ("0m0.0s", Err("Invalid interval '0m0.0s': interval cannot be zero")),
            ("0m0.00s", Err("Invalid interval '0m0.00s': interval cannot be zero")),
            ("0m0.000s", Err("Invalid interval '0m0.000s': interval cannot be zero")),
            ("0m0.0000s", Err("Invalid interval '0m0.0000s': maximum precision is millisecond")),
            ("1m0s", Ok(60 * 1000)),
            ("1m0.0s", Ok(60 * 1000)),
            ("1m0.01s", Ok(60 * 1000 + 10)),
            ("1m0.001s", Ok(60 * 1000 + 1)),
            ("1m0.0010s", Err("Invalid interval '1m0.0010s': maximum precision is millisecond")),
            ("1m0.12s", Ok(60 * 1000 + 120)),
            ("1m0.123s", Ok(60 * 1000 + 123)),
            ("1m0.1234s", Err("Invalid interval '1m0.1234s': maximum precision is millisecond")),
            ("2m3.4s", Ok((2 * 60 + 3) * 1_000 + 400)),
            ("34m56.78s", Ok((34 * 60 + 56) * 1_000 + 780)),
            ("456m7.890s", Ok((456 * 60 + 7) * 1_000 + 890)),
            ("456m7.891s", Ok((456 * 60 + 7) * 1_000 + 891)),
            // hours
            ("0h", Err("Invalid interval '0h': interval cannot be zero")),
            ("1h", Ok(3600 * 1000)),
            ("123h", Ok(123 * 3600 * 1000)),
            // hours + seconds
            ("0h0s", Err("Invalid interval '0h0s': interval cannot be zero")),
            ("0h1s", Ok(1000)),
            ("1h0s", Ok(3600 * 1000)),
            ("1h1s", Ok((3600 + 1) * 1000)),
            ("1h2s", Ok((3600 + 2) * 1000)),
            ("1h2.3s", Ok((3600 + 2) * 1000 + 300)),
            ("1h2.34s", Ok((3600 + 2) * 1000 + 340)),
            ("1h2.345s", Ok((3600 + 2) * 1000 + 345)),
            ("1h2.3456s", Err("Invalid interval '1h2.3456s': maximum precision is millisecond")),
            // hours + minutes
            ("0h0m", Err("Invalid interval '0h0m': interval cannot be zero")),
            ("0h1m", Ok(60 * 1000)),
            ("1h0m", Ok(3600 * 1000)),
            ("1h1m", Ok((3600 + 60) * 1000)),
            ("1h2m", Ok((3600 + 2 * 60) * 1000)),
            // hours + minutes + seconds
            ("1h2m3s", Ok((3600 + 2 * 60 + 3) * 1000)),
            ("1h2m3.4s", Ok((3600 + 2 * 60 + 3) * 1000 + 400)),
            ("1h2m3.45s", Ok((3600 + 2 * 60 + 3) * 1000 + 450)),
            ("1h2m3.456s", Ok((3600 + 2 * 60 + 3) * 1000 + 456)),
            ("1h2m3.4567s", Err("Invalid interval '1h2m3.4567s': maximum precision is millisecond")),
            // days
            ("0d", Err("Invalid interval '0d': interval cannot be zero")),
            ("1d", Ok(86400 * 1000)),
            ("123d", Ok(123 * 86400 * 1000)),
            // days + seconds
            ("0d0s", Err("Invalid interval '0d0s': interval cannot be zero")),
            ("0d1s", Ok(1000)),
            ("1d0s", Ok(86400 * 1000)),
            ("1d1s", Ok((86400 + 1) * 1000)),
            ("2d3s", Ok((2 * 86400 + 3) * 1000)),
            ("2d3.4s", Ok((2 * 86400 + 3) * 1000 + 400)),
            ("2d3.45s", Ok((2 * 86400 + 3) * 1000 + 450)),
            ("2d3.456s", Ok((2 * 86400 + 3) * 1000 + 456)),
            ("2d3.4567s", Err("Invalid interval '2d3.4567s': maximum precision is millisecond")),
            // days + minutes
            ("0d0m", Err("Invalid interval '0d0m': interval cannot be zero")),
            ("0d1m", Ok(60 * 1000)),
            ("1d0m", Ok(86400 * 1000)),
            ("1d1m", Ok((86400 + 60) * 1000)),
            ("2d3m", Ok((2 * 86400 + 3 * 60) * 1000)),
            // days + minutes + seconds
            ("0d0m0s", Err("Invalid interval '0d0m0s': interval cannot be zero")),
            ("0d0m0.0s", Err("Invalid interval '0d0m0.0s': interval cannot be zero")),
            ("0d0m1s", Ok(1000)),
            ("0d1m0s", Ok(60 * 1000)),
            ("1d0m0s", Ok(86400 * 1000)),
            ("1d1m0s", Ok((86400 + 60) * 1000)),
            ("2d3m4s", Ok((2 * 86400 + 3 * 60 + 4) * 1000)),
            ("2d3m4.5s", Ok((2 * 86400 + 3 * 60 + 4) * 1000 + 500)),
            ("2d3m4.56s", Ok((2 * 86400 + 3 * 60 + 4) * 1000 + 560)),
            ("2d3m4.567s", Ok((2 * 86400 + 3 * 60 + 4) * 1000 + 567)),
            ("2d3m4.5678s", Err("Invalid interval '2d3m4.5678s': maximum precision is millisecond")),
            // days + hours
            ("0d0h", Err("Invalid interval '0d0h': interval cannot be zero")),
            ("0d1h", Ok(3600 * 1000)),
            ("1d0h", Ok(86400 * 1000)),
            ("1d1h", Ok((86400 + 3600) * 1000)),
            ("2d3h", Ok((2 * 86400 + 3 * 3600) * 1000)),
            // days + hours + seconds
            ("0d0h0s", Err("Invalid interval '0d0h0s': interval cannot be zero")),
            ("0d0h0.0s", Err("Invalid interval '0d0h0.0s': interval cannot be zero")),
            ("0d0h1s", Ok(1000)),
            ("0d1h0s", Ok(3600 * 1000)),
            ("1d0h0s", Ok(86400 * 1000)),
            ("1d1h0s", Ok((86400 + 3600) * 1000)),
            ("2d3h4s", Ok((2 * 86400 + 3 * 3600 + 4) * 1000)),
            ("2d3h4.5s", Ok((2 * 86400 + 3 * 3600 + 4) * 1000 + 500)),
            ("2d3h4.56s", Ok((2 * 86400 + 3 * 3600 + 4) * 1000 + 560)),
            ("2d3h4.567s", Ok((2 * 86400 + 3 * 3600 + 4) * 1000 + 567)),
            ("2d3h4.5678s", Err("Invalid interval '2d3h4.5678s': maximum precision is millisecond")),
            // days + hours + minutes
            ("0d0h0m", Err("Invalid interval '0d0h0m': interval cannot be zero")),
            ("0d0h1m", Ok(60 * 1000)),
            ("0d1h0m", Ok(3600 * 1000)),
            ("1d0h0m", Ok(86400 * 1000)),
            ("2d3h4m", Ok((2 * 86400 + 3 * 3600 + 4 * 60) * 1000)),
            // days + hours + minutes + seconds
            ("0d0h0m0s", Err("Invalid interval '0d0h0m0s': interval cannot be zero")),
            ("0d0h0m0.0s", Err("Invalid interval '0d0h0m0.0s': interval cannot be zero")),
            ("0d0h0m0.00s", Err("Invalid interval '0d0h0m0.00s': interval cannot be zero")),
            ("0d0h0m0.000s", Err("Invalid interval '0d0h0m0.000s': interval cannot be zero")),
            ("0d0h0m0.0000s", Err("Invalid interval '0d0h0m0.0000s': maximum precision is millisecond")),
            ("0d0h0m1s", Ok(1000)),
            ("0d0h1m0s", Ok(60 * 1000)),
            ("0d1h0m0s", Ok(3600 * 1000)),
            ("1d0h0m0s", Ok(86400 * 1000)),
            ("1d2h3m4s", Ok((86400 + 2 * 3600 + 3 * 60 + 4) * 1000)),
            ("1d2h3m4.5s", Ok((86400 + 2 * 3600 + 3 * 60 + 4) * 1000 + 500)),
            ("1d2h3m4.56s", Ok((86400 + 2 * 3600 + 3 * 60 + 4) * 1000 + 560)),
            ("1d2h3m4.567s", Ok((86400 + 2 * 3600 + 3 * 60 + 4) * 1000 + 567)),
            ("1d2h3m4.5678s", Err("Invalid interval '1d2h3m4.5678s': maximum precision is millisecond")),
            ("2d3s", Ok((2 * 86400 + 3) * 1000)),
            ("2d3.4s", Ok((2 * 86400 + 3) * 1000 + 400)),
            ("2d3h4.56s", Ok((2 * 86400 + 3 * 3600 + 4) * 1000 + 560)),
            ("1d1.123s", Ok((86400 + 1) * 1000 + 123)),
            ("1d1.1234s", Err("Invalid interval '1d1.1234s': maximum precision is millisecond")),
            // out of order units
            ("1h1d", Err("Invalid interval '1h1d': unrecognized format")),
            ("1m1d", Err("Invalid interval '1m1d': unrecognized format")),
            ("1m1h", Err("Invalid interval '1m1h': unrecognized format")),
            ("1s1d", Err("Invalid interval '1s1d': unrecognized format")),
            ("1s1h", Err("Invalid interval '1s1h': unrecognized format")),
            ("1s1m", Err("Invalid interval '1s1m': unrecognized format")),
            // integer overflow
            ("18446744073709551s", Ok(18446744073709551000)),
            ("18446744073709551.999s", Err("Invalid interval '18446744073709551.999s': interval is too large")),
            ("18446744073709552s", Err("Invalid interval '18446744073709552s': interval is too large")),
            ("307445734561825m51s", Ok(18446744073709551000)),
            ("307445734561825m52s", Err("Invalid interval '307445734561825m52s': interval is too large")),
            ("307445734561825m", Ok(18446744073709500000)),
            ("307445734561826m", Err("Invalid interval '307445734561826m': interval is too large")),
            ("5124095576030h25m", Ok(18446744073709500000)),
            ("5124095576030h26m", Err("Invalid interval '5124095576030h26m': interval is too large")),
            ("5124095576030h", Ok(18446744073708000000)),
            ("5124095576031h", Err("Invalid interval '5124095576031h': interval is too large")),
            ("213503982334d14h", Ok(18446744073708000000)),
            ("213503982334d15h", Err("Invalid interval '213503982334d15h': interval is too large")),
            ("213503982334d", Ok(18446744073657600000)),
            ("213503982335d", Err("Invalid interval '213503982335d': interval is too large")),
        ];

        for (input, expected) in tests_cases {
            let actual = parse_interval_as_ms(input);
            let expected = expected.map_err(|e| e.to_string());

            assert_eq!(actual, expected, "input: {input}");
        }
    }

    #[test]
    fn test_parse_concurrency() {
        let test_cases = [
            ("", Err("Invalid concurrency value: ''")),
            ("-1", Err("Invalid concurrency value: '-1'")),
            (" 1", Err("Invalid concurrency value: ' 1'")),
            ("1 ", Err("Invalid concurrency value: '1 '")),
            ("1 1", Err("Invalid concurrency value: '1 1'")),
            ("1.", Err("Invalid concurrency value: '1.'")),
            ("1.0", Err("Invalid concurrency value: '1.0'")),
            ("0", Err("Invalid concurrency: value 0 is not in the range 1–1000")),
            ("1", Ok(1)),
            ("2", Ok(2)),
            ("3", Ok(3)),
            ("10", Ok(10)),
            ("100", Ok(100)),
            ("1000", Ok(1000)),
            ("1001", Err("Invalid concurrency: value 1001 is not in the range 1–1000")),
            ("9999999999", Err("Invalid concurrency: value 9999999999 is not in the range 1–1000")),
            ("abc", Err("Invalid concurrency value: 'abc'")),
        ];

        for (input, expected) in test_cases {
            let actual = parse_concurrency(input);
            let expected = expected.map_err(|e| e.to_string());

            assert_eq!(actual, expected, "input: {input}");
        }
    }
}
