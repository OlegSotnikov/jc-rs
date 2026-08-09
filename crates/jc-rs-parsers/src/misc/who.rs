//! Parser for `who` command output.

use chrono::{Local, NaiveDateTime, TimeZone};
use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use regex::Regex;
use serde_json::{Map, Value};
use std::sync::OnceLock;

pub struct WhoParser;

static INFO: ParserInfo = ParserInfo {
    name: "who",
    argument: "--who",
    version: "1.6.0",
    description: "Converts `who` command output to JSON",
    author: "jc-rs contributors",
    author_email: "os@g1sw.com",
    compatible: &[
        Platform::Linux,
        Platform::Darwin,
        Platform::FreeBSD,
        Platform::Aix,
    ],
    tags: &[Tag::Command],
    magic_commands: &["who"],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static WHO_PARSER: WhoParser = WhoParser;

inventory::submit! {
    ParserEntry::new(&WHO_PARSER)
}

static PROCESS_RE: OnceLock<Regex> = OnceLock::new();
static MONTH_RE: OnceLock<Regex> = OnceLock::new();

fn get_process_re() -> &'static Regex {
    PROCESS_RE.get_or_init(|| {
        // Matches: user PROCESS pts/N (the second token is a process name, not tty)
        Regex::new(r"^\S+\s+[^ +\-]+\s+pts/\d+\s").unwrap()
    })
}

fn get_month_re() -> &'static Regex {
    MONTH_RE.get_or_init(|| {
        // Matches month abbreviations like Jan, Feb, Mar, Apr, May, Jun, Jul, Aug, Sep, Oct, Nov, Dec
        Regex::new(r"^[JFMASOND][aepuco][nbrynlgptvc]").unwrap()
    })
}

/// Try to convert a time string to a naive epoch (local time interpretation).
/// Returns null if parsing fails.
fn time_to_epoch(time_str: &str) -> Value {
    // Try YYYY-MM-DD HH:MM format
    let fmts = ["%Y-%m-%d %H:%M", "%Y-%m-%d %H:%M:%S"];
    for fmt in &fmts {
        if let Ok(dt) = NaiveDateTime::parse_from_str(time_str, fmt)
            && let Some(local_dt) = Local.from_local_datetime(&dt).single()
        {
            return Value::Number(local_dt.timestamp().into());
        }
    }
    Value::Null
}

impl Parser for WhoParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Array(vec![]));
        }

        let mut result = Vec::new();

        for line in input.lines() {
            let line = line.trim_end(); // Keep leading spaces for now
            if line.trim().is_empty() {
                continue;
            }

            let mut tokens: Vec<String> = line.split_whitespace().map(|s| s.to_string()).collect();
            if tokens.is_empty() {
                continue;
            }

            let mut obj: Map<String, Value> = Map::new();

            // Skip header lines
            let header_check: String = tokens
                .iter()
                .take(3)
                .cloned()
                .collect::<Vec<_>>()
                .join("")
                .to_uppercase();
            if header_check == "NAMELINETIME" || header_check == "USERLINEWHEN" {
                continue;
            }

            // Handle special event lines
            if tokens[0] == "reboot" {
                obj.insert("event".to_string(), Value::String("reboot".to_string()));
                if tokens.len() >= 5 {
                    let time_str = tokens[2..5].join(" ");
                    let epoch = time_to_epoch(&time_str);
                    obj.insert("time".to_string(), Value::String(time_str));
                    obj.insert("epoch".to_string(), epoch);
                }
                if tokens.len() >= 7
                    && let Ok(pid) = tokens[6].parse::<i64>()
                {
                    obj.insert("pid".to_string(), Value::Number(pid.into()));
                }
                result.push(obj);
                continue;
            }

            if tokens.len() >= 2 && tokens[0] == "system" && tokens[1] == "boot" {
                obj.insert("event".to_string(), Value::String("reboot".to_string()));
                if tokens.len() >= 4 {
                    let time_str = tokens[2..4].join(" ");
                    let epoch = time_to_epoch(&time_str);
                    obj.insert("time".to_string(), Value::String(time_str));
                    obj.insert("epoch".to_string(), epoch);
                }
                result.push(obj);
                continue;
            }

            if tokens[0] == "LOGIN" {
                obj.insert("event".to_string(), Value::String("login".to_string()));
                if tokens.len() > 1 {
                    obj.insert("tty".to_string(), Value::String(tokens[1].clone()));
                }
                if tokens.len() >= 4 {
                    let time_str = tokens[2..4].join(" ");
                    let epoch = time_to_epoch(&time_str);
                    obj.insert("time".to_string(), Value::String(time_str));
                    obj.insert("epoch".to_string(), epoch);
                }
                if tokens.len() >= 5
                    && let Ok(pid) = tokens[4].parse::<i64>()
                {
                    obj.insert("pid".to_string(), Value::Number(pid.into()));
                }
                if tokens.len() > 5 {
                    obj.insert("comment".to_string(), Value::String(tokens[5..].join(" ")));
                }
                result.push(obj);
                continue;
            }

            if tokens[0] == "run-level" {
                obj.insert("event".to_string(), Value::String(tokens[..2].join(" ")));
                if tokens.len() >= 4 {
                    let time_str = tokens[2..4].join(" ");
                    let epoch = time_to_epoch(&time_str);
                    obj.insert("time".to_string(), Value::String(time_str));
                    obj.insert("epoch".to_string(), epoch);
                }
                result.push(obj);
                continue;
            }

            // Skip lines where second token is "run-level"
            if tokens.len() >= 2 && tokens[1] == "run-level" {
                continue;
            }

            // Lines where first token is a tty (pts/N format without user)
            // Note: these lines often start with leading spaces in `who -a` output
            if tokens[0].starts_with("pts/") {
                obj.insert("tty".to_string(), Value::String(tokens[0].clone()));
                if tokens.len() >= 3 {
                    let time_str = tokens[1..3].join(" ");
                    let epoch = time_to_epoch(&time_str);
                    obj.insert("time".to_string(), Value::String(time_str));
                    obj.insert("epoch".to_string(), epoch);
                }
                if tokens.len() >= 4
                    && let Ok(pid) = tokens[3].parse::<i64>()
                {
                    obj.insert("pid".to_string(), Value::Number(pid.into()));
                }
                if tokens.len() > 4 {
                    obj.insert("comment".to_string(), Value::String(tokens[4..].join(" ")));
                }
                result.push(obj);
                continue;
            }

            // Check for "user process pts/N ..." pattern
            let mut process: Option<String> = None;
            if tokens.len() >= 3 && get_process_re().is_match(line) {
                process = Some(tokens.remove(1));
            }

            // user
            let user = tokens.remove(0);
            obj.insert("user".to_string(), Value::String(user));

            // writeable_tty (+/-/?)
            if !tokens.is_empty() && (tokens[0] == "+" || tokens[0] == "-" || tokens[0] == "?") {
                obj.insert("writeable_tty".to_string(), Value::String(tokens.remove(0)));
            }

            // tty
            if tokens.is_empty() {
                result.push(obj);
                continue;
            }
            let tty = tokens.remove(0);
            obj.insert("tty".to_string(), Value::String(tty));

            // process (if detected earlier)
            if let Some(p) = process {
                obj.insert("process".to_string(), Value::String(p));
            }

            // Parse time
            if tokens.is_empty() {
                result.push(obj);
                continue;
            }

            let time_str = if get_month_re().is_match(&tokens[0]) && tokens.len() >= 3 {
                // Month-based format: Mon DD HH:MM
                let t = tokens[..3].join(" ");
                tokens.drain(0..3);
                t
            } else if tokens.len() >= 2 {
                // Date-based format: YYYY-MM-DD HH:MM
                let t = tokens[..2].join(" ");
                tokens.drain(0..2);
                t
            } else {
                tokens.remove(0)
            };
            obj.insert("time".to_string(), Value::String(time_str.clone()));

            // Remaining tokens after time
            if !tokens.is_empty() {
                let rest = tokens.join(" ");

                if tokens.len() > 1 && rest.starts_with('(') && rest.ends_with(')') {
                    // From field: "(host)" possibly with spaces
                    obj.insert(
                        "from".to_string(),
                        Value::String(rest[1..rest.len() - 1].to_string()),
                    );
                } else if tokens.len() == 1 {
                    let tok = &tokens[0];
                    if tok.starts_with('(') {
                        obj.insert(
                            "from".to_string(),
                            Value::String(
                                tok.trim_start_matches('(')
                                    .trim_end_matches(')')
                                    .to_string(),
                            ),
                        );
                    } else {
                        // idle field
                        obj.insert("idle".to_string(), Value::String(tok.clone()));
                    }
                } else {
                    // idle + pid + optional from/comment
                    let idle = tokens.remove(0);
                    obj.insert("idle".to_string(), Value::String(idle));

                    if !tokens.is_empty() {
                        let pid_str = &tokens[0];
                        if let Ok(pid) = pid_str.parse::<i64>() {
                            obj.insert("pid".to_string(), Value::Number(pid.into()));
                            tokens.remove(0);
                        }
                    }

                    if !tokens.is_empty() {
                        let tok = &tokens[0];
                        if tok.starts_with('(') {
                            let from_str = tokens.join(" ");
                            let from = from_str
                                .trim_start_matches('(')
                                .trim_end_matches(')')
                                .to_string();
                            obj.insert("from".to_string(), Value::String(from));
                        } else {
                            obj.insert("comment".to_string(), Value::String(tokens.join(" ")));
                        }
                    }
                }
            }

            // Compute epoch
            let epoch = time_to_epoch(&time_str);
            obj.insert("epoch".to_string(), epoch);

            result.push(obj);
        }

        Ok(ParseOutput::Array(result))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check_who_fixture(input: &str, expected_json: &str) {
        let parser = WhoParser;
        let result = parser.parse(input, false).unwrap();
        let expected: Vec<serde_json::Value> = serde_json::from_str(expected_json).unwrap();
        match result {
            ParseOutput::Array(arr) => {
                assert_eq!(arr.len(), expected.len(), "row count mismatch");
                for (i, (got, exp)) in arr.iter().zip(expected.iter()).enumerate() {
                    // Compare all fields except epoch (timezone-dependent)
                    for field in &[
                        "user",
                        "tty",
                        "time",
                        "from",
                        "event",
                        "writeable_tty",
                        "process",
                        "idle",
                        "comment",
                    ] {
                        let g = got.get(*field).unwrap_or(&Value::Null);
                        let e = exp.get(*field).unwrap_or(&Value::Null);
                        assert_eq!(g, e, "row {} field '{}' mismatch", i, field);
                    }
                    // For pid, compare if present in expected
                    if exp.get("pid") != Some(&Value::Null)
                        && let Some(exp_pid) = exp.get("pid")
                    {
                        assert_eq!(
                            got.get("pid").unwrap_or(&Value::Null),
                            exp_pid,
                            "row {} field 'pid' mismatch",
                            i
                        );
                    }
                    // epoch: if expected is null, we should also be null
                    if let Some(Value::Null) = exp.get("epoch") {
                        assert_eq!(
                            got.get("epoch").unwrap_or(&Value::Null),
                            &Value::Null,
                            "row {} epoch should be null",
                            i
                        );
                    }
                    // If expected epoch is non-null, just check our epoch is also non-null
                    if let Some(exp_epoch) = exp.get("epoch")
                        && exp_epoch != &Value::Null
                    {
                        let got_epoch = got.get("epoch").unwrap_or(&Value::Null);
                        assert!(
                            got_epoch != &Value::Null,
                            "row {} epoch should not be null",
                            i
                        );
                    }
                }
            }
            _ => panic!("expected Array"),
        }
    }

    #[test]
    fn test_who_ubuntu1804() {
        check_who_fixture(
            include_str!("../../../../tests/fixtures/ubuntu-18.04/who.out"),
            include_str!("../../../../tests/fixtures/ubuntu-18.04/who.json"),
        );
    }

    #[test]
    fn test_who_centos77() {
        check_who_fixture(
            include_str!("../../../../tests/fixtures/centos-7.7/who.out"),
            include_str!("../../../../tests/fixtures/centos-7.7/who.json"),
        );
    }

    #[test]
    fn test_who_osx() {
        check_who_fixture(
            include_str!("../../../../tests/fixtures/osx-10.14.6/who.out"),
            include_str!("../../../../tests/fixtures/osx-10.14.6/who.json"),
        );
    }

    #[test]
    fn test_who_debian13() {
        check_who_fixture(
            include_str!("../../../../tests/fixtures/debian13/who.out"),
            include_str!("../../../../tests/fixtures/debian13/who.json"),
        );
    }

    #[test]
    fn test_who_empty() {
        let parser = WhoParser;
        let result = parser.parse("", false).unwrap();
        if let ParseOutput::Array(arr) = result {
            assert!(arr.is_empty());
        } else {
            panic!("expected Array");
        }
    }

    #[test]
    fn test_who_login_screen() {
        // `from` reads "login screen" for a seat, and a plain local login has
        // no `from` at all. The epoch is timezone-dependent, so this asserts
        // structure rather than a number.
        let input = include_str!("../../../../tests/fixtures/generic/who-login-screen.out");
        let ParseOutput::Array(rows) = WhoParser.parse(input, false).unwrap() else {
            panic!("expected an array");
        };
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0]["user"], "atemu");
        assert_eq!(rows[0]["tty"], "seat0");
        assert_eq!(rows[0]["from"], "login screen");
        assert!(!rows[2].contains_key("from"));
    }
}
