//! Parser for `who` command output.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use jc_rs_utils::parse_timestamp;
use serde_json::{Map, Value};

pub struct WhoParser;

static INFO: ParserInfo = ParserInfo {
    name: "who",
    argument: "--who",
    version: "1.7.0",
    description: "Converts `who` command output to JSON",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[Platform::Linux, Platform::Darwin, Platform::FreeBSD],
    tags: &[Tag::Command, Tag::Slurpable],
    magic_commands: &["who"],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static WHO_PARSER: WhoParser = WhoParser;

// Note: misc/who.rs is the canonical registration; this file is kept for its test suite only.
// inventory::submit! { ParserEntry::new(&WHO_PARSER) }

impl Parser for WhoParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        let rows = parse_who(input);
        Ok(ParseOutput::Array(rows))
    }
}

fn parse_who(input: &str) -> Vec<Map<String, Value>> {
    let mut output = Vec::new();

    for line in input.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let mut tokens: Vec<&str> = line.split_whitespace().collect();

        // Skip header lines like "NAME LINE TIME" or "USER LINE WHEN"
        if tokens.len() >= 3 {
            let first3 = format!(
                "{}{}{}",
                tokens[0].to_uppercase(),
                tokens[1].to_uppercase(),
                tokens[2].to_uppercase()
            );
            if first3 == "NAMELINETIME" || first3 == "USERLINEWHEN" {
                continue;
            }
        }

        if tokens.is_empty() {
            continue;
        }

        let mut record = Map::new();

        // Handle special event lines
        if tokens[0] == "reboot" {
            // Mac: reboot ~ boot-time pid
            record.insert("event".to_string(), Value::String("reboot".to_string()));
            if tokens.len() >= 5 {
                let time_str = tokens[2..5].join(" ");
                if let Some(pid) = tokens.get(6).and_then(|s| s.parse::<i64>().ok()) {
                    record.insert("pid".to_string(), Value::Number(pid.into()));
                }
                add_time_and_epoch(&mut record, &time_str);
            }
            output.push(record);
            continue;
        }

        if tokens.len() >= 2 && format!("{}{}", tokens[0], tokens[1]) == "systemboot" {
            // Linux: system boot date time
            record.insert("event".to_string(), Value::String("reboot".to_string()));
            if tokens.len() >= 4 {
                let time_str = tokens[2..4].join(" ");
                add_time_and_epoch(&mut record, &time_str);
            }
            output.push(record);
            continue;
        }

        if tokens[0] == "LOGIN" {
            record.insert("event".to_string(), Value::String("login".to_string()));
            if let Some(tty) = tokens.get(1) {
                record.insert("tty".to_string(), Value::String(tty.to_string()));
            }
            if tokens.len() >= 4 {
                let time_str = tokens[2..4].join(" ");
                add_time_and_epoch(&mut record, &time_str);
            }
            if let Some(pid_str) = tokens.get(4) {
                if let Ok(pid) = pid_str.parse::<i64>() {
                    record.insert("pid".to_string(), Value::Number(pid.into()));
                }
            }
            if tokens.len() > 5 {
                let comment = tokens[5..].join(" ");
                record.insert("comment".to_string(), Value::String(comment));
            }
            output.push(record);
            continue;
        }

        if tokens[0] == "run-level" {
            let event = tokens[..2.min(tokens.len())].join(" ");
            record.insert("event".to_string(), Value::String(event));
            if tokens.len() >= 4 {
                let time_str = tokens[2..4].join(" ");
                add_time_and_epoch(&mut record, &time_str);
            }
            output.push(record);
            continue;
        }

        // Mac run-level (ignore)
        if tokens.len() >= 2 && tokens[1] == "run-level" {
            continue;
        }

        // pts lines with no user
        if tokens[0].starts_with("pts/") {
            record.insert("tty".to_string(), Value::String(tokens[0].to_string()));
            if tokens.len() >= 3 {
                let time_str = tokens[1..3].join(" ");
                add_time_and_epoch(&mut record, &time_str);
            }
            if let Some(pid_str) = tokens.get(3) {
                if let Ok(pid) = pid_str.parse::<i64>() {
                    record.insert("pid".to_string(), Value::Number(pid.into()));
                }
            }
            if tokens.len() > 4 {
                let comment = tokens[4..].join(" ");
                record.insert("comment".to_string(), Value::String(comment));
            }
            output.push(record);
            continue;
        }

        // Check for process name pattern: "user process pts/N ..."
        // Detect: user non-pts-non-flag pts/N ...
        let mut user_process: Option<String> = None;
        if tokens.len() >= 3 {
            let t1 = tokens[1];
            // process name: not starting with +/-/?, and not a tty pattern, not a date
            if !t1.starts_with('+')
                && !t1.starts_with('-')
                && !t1.starts_with('?')
                && (tokens[2].starts_with("pts/") || tokens[2].contains('/'))
                && !t1.contains(':')
            {
                user_process = Some(t1.to_string());
                tokens.remove(1);
            }
        }

        // User logins
        let user = tokens[0].to_string();
        record.insert("user".to_string(), Value::String(user));
        tokens = tokens[1..].to_vec();

        // writeable tty indicator
        if let Some(&t) = tokens.first() {
            if t == "+" || t == "-" || t == "?" {
                record.insert("writeable_tty".to_string(), Value::String(t.to_string()));
                tokens = tokens[1..].to_vec();
            }
        }

        // tty
        if let Some(tty) = tokens.first() {
            record.insert("tty".to_string(), Value::String(tty.to_string()));
            tokens = tokens[1..].to_vec();
        }

        if let Some(proc_name) = user_process {
            record.insert("process".to_string(), Value::String(proc_name));
        }

        // Time: detect mac (Mon Jan DD) vs linux (YYYY-MM-DD HH:MM or MMon DD)
        if tokens.is_empty() {
            output.push(record);
            continue;
        }

        // Mac time: starts with month abbreviation like "Jan", "Feb", etc.
        let is_mac_time = tokens
            .first()
            .map(|t| {
                matches!(
                    t.to_lowercase().as_str(),
                    "jan"
                        | "feb"
                        | "mar"
                        | "apr"
                        | "may"
                        | "jun"
                        | "jul"
                        | "aug"
                        | "sep"
                        | "oct"
                        | "nov"
                        | "dec"
                )
            })
            .unwrap_or(false);

        let time_str = if is_mac_time && tokens.len() >= 3 {
            let t = tokens[..3].join(" ");
            tokens = tokens[3..].to_vec();
            t
        } else if tokens.len() >= 2 {
            let t = tokens[..2].join(" ");
            tokens = tokens[2..].to_vec();
            t
        } else {
            let t = tokens[0].to_string();
            tokens = tokens[1..].to_vec();
            t
        };

        add_time_and_epoch(&mut record, &time_str);

        // Remaining tokens: check for "(from)" pattern
        if !tokens.is_empty() {
            let rest = tokens.join(" ");
            if rest.starts_with('(') && rest.ends_with(')') {
                let from = rest[1..rest.len() - 1].to_string();
                record.insert("from".to_string(), Value::String(from));
            } else if tokens.len() == 1 {
                // Single token: could be idle, pid, etc.
                let t = tokens[0];
                if let Ok(pid) = t.parse::<i64>() {
                    record.insert("pid".to_string(), Value::Number(pid.into()));
                } else {
                    record.insert("idle".to_string(), Value::String(t.to_string()));
                }
            } else {
                // Multiple remaining tokens
                // Check for idle pid comment pattern
                let mut idx = 0;
                if let Some(idle) = tokens.get(idx) {
                    if !idle.starts_with('(') {
                        record.insert("idle".to_string(), Value::String(idle.to_string()));
                        idx += 1;
                    }
                }
                if let Some(pid_str) = tokens.get(idx) {
                    if let Ok(pid) = pid_str.parse::<i64>() {
                        record.insert("pid".to_string(), Value::Number(pid.into()));
                        idx += 1;
                    }
                }
                if idx < tokens.len() {
                    let remaining = tokens[idx..].join(" ");
                    if remaining.starts_with('(') && remaining.ends_with(')') {
                        // Parenthesized value is the remote host → "from"
                        let host = remaining[1..remaining.len() - 1].to_string();
                        record.insert("from".to_string(), Value::String(host));
                    } else {
                        record.insert("comment".to_string(), Value::String(remaining));
                    }
                }
            }
        }

        output.push(record);
    }

    output
}

fn add_time_and_epoch(record: &mut Map<String, Value>, time_str: &str) {
    record.insert("time".to_string(), Value::String(time_str.to_string()));
    let ts = parse_timestamp(time_str, None);
    record.insert(
        "epoch".to_string(),
        ts.naive_epoch
            .map(|e| Value::Number(e.into()))
            .unwrap_or(Value::Null),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_who_login_screen_smoke() {
        // The epoch value is timezone-dependent (naive), so we just check structure
        let input = include_str!("../../../../tests/fixtures/generic/who-login-screen.out");
        let parser = WhoParser;
        let result = parser.parse(input, false).unwrap();
        if let ParseOutput::Array(arr) = result {
            assert_eq!(arr.len(), 3);
            assert_eq!(
                arr[0].get("user"),
                Some(&Value::String("atemu".to_string()))
            );
            assert_eq!(arr[0].get("tty"), Some(&Value::String("seat0".to_string())));
            // from field should be "login screen"
            assert_eq!(
                arr[0].get("from"),
                Some(&Value::String("login screen".to_string()))
            );
            // third record has no from
            assert!(arr[2].get("from").is_none());
        } else {
            panic!("Expected Array");
        }
    }
}
