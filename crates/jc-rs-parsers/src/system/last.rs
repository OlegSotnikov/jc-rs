//! Parser for `last` and `lastb` command output.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use jc_rs_utils::parse_timestamp;
use serde_json::{Map, Value};

pub struct LastParser;

static INFO: ParserInfo = ParserInfo {
    name: "last",
    argument: "--last",
    version: "1.9.0",
    description: "Converts `last` and `lastb` command output to JSON",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[Platform::Linux, Platform::Darwin, Platform::FreeBSD],
    tags: &[Tag::Command],
    magic_commands: &["last", "lastb"],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static LAST_PARSER: LastParser = LastParser;

inventory::submit! {
    ParserEntry::new(&LAST_PARSER)
}

/// Returns true if a two-token string looks like "Mon Feb"
fn is_date_two(a: &str, b: &str) -> bool {
    let weekdays = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
    let months = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    weekdays.contains(&a) && months.contains(&b)
}

/// Returns true if tokens at (i, i+1) look like "HH:MM:SS YYYY"
fn is_last_f_at(tokens: &[String], i: usize) -> bool {
    if i + 1 >= tokens.len() {
        return false;
    }
    let t0 = &tokens[i];
    let t1 = &tokens[i + 1];
    t0.len() == 8
        && t0.chars().nth(2) == Some(':')
        && t0.chars().nth(5) == Some(':')
        && t1.len() == 4
        && t1.chars().all(|c| c.is_ascii_digit())
}

/// Normalize duration "1+18:47" → "42:47"
fn normalize_duration(dur: &str) -> String {
    if let Some(plus_pos) = dur.find('+') {
        let days_str = &dur[..plus_pos];
        let rest = &dur[plus_pos + 1..];
        if let Ok(days) = days_str.parse::<i64>() {
            if let Some(colon_pos) = rest.find(':') {
                let hours_str = &rest[..colon_pos];
                let minutes_str = &rest[colon_pos + 1..];
                if let Ok(hours) = hours_str.parse::<i64>() {
                    let total_hours = days * 24 + hours;
                    return format!("{}:{}", total_hours, minutes_str);
                }
            }
        }
    }
    dur.to_string()
}

impl Parser for LastParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Array(Vec::new()));
        }

        let mut result = Vec::new();

        for raw_line in input.lines() {
            let line = raw_line.trim_end();
            if line.trim().is_empty() {
                continue;
            }

            // Skip summary lines
            let trimmed = line.trim();
            if trimmed.starts_with("wtmp begins ")
                || trimmed.starts_with("btmp begins ")
                || trimmed.starts_with("utx.log begins ")
            {
                continue;
            }

            // Apply jc's substitutions before splitting
            let line = line
                .replace("boot time", "boot_time")
                .replace("  still logged in", "- still_logged_in")
                .replace("  gone - no logout", "- gone_-_no_logout");

            let mut tokens: Vec<String> = line.split_whitespace().map(String::from).collect();
            if tokens.is_empty() {
                continue;
            }

            // Insert "-" before date tokens when no explicit hostname separator
            // jc checks if tokens[2]+tokens[3] looks like "Mon Feb"
            if tokens.len() >= 4 && is_date_two(&tokens[2], &tokens[3]) {
                tokens.insert(2, "-".to_string());
            }

            // FreeBSD boot_time: insert "~" (tty) and "-" (hostname) before date
            if tokens[0] == "boot_time" && (tokens.len() < 2 || tokens[1] != "~") {
                tokens.insert(1, "-".to_string());
                tokens.insert(1, "~".to_string());
            }

            let user_raw = tokens[0].clone();
            let user = if user_raw == "boot_time" {
                "boot time".to_string()
            } else {
                user_raw.clone()
            };

            // Fix -x options: runlevel "(to X level)" and reboot/shutdown "system boot/down"
            if user_raw == "runlevel" && tokens.len() > 1 && tokens[1] == "(to" && tokens.len() > 3
            {
                let extra2 = tokens.remove(2);
                let extra3 = tokens.remove(2);
                tokens[1] = format!("{} {} {}", tokens[1], extra2, extra3);
            } else if (user_raw == "reboot" || user_raw == "shutdown")
                && tokens.len() > 2
                && tokens[1] == "system"
            {
                let extra = tokens.remove(2);
                tokens[1] = format!("{} {}", tokens[1], extra);
            }

            let tty = tokens.get(1).cloned().unwrap_or_default();
            let hostname = tokens.get(2).cloned().unwrap_or_default();

            let mut obj = Map::new();
            obj.insert("user".to_string(), Value::String(user));

            // tty: "~" → null
            if tty == "~" {
                obj.insert("tty".to_string(), Value::Null);
            } else {
                obj.insert("tty".to_string(), Value::String(tty));
            }

            // hostname: "-" → null; ":N" → "CONSOLEN"
            let hostname_val = if hostname == "-" {
                Value::Null
            } else if hostname.starts_with(':') {
                Value::String(format!("CONSOLE{}", hostname))
            } else {
                Value::String(hostname)
            };
            obj.insert("hostname".to_string(), hostname_val);

            // Determine if -F format by checking tokens[6] and tokens[7]
            let rest = 3usize; // index of first login token
            let is_f = tokens.len() > 7 && is_last_f_at(&tokens, rest + 3);

            if is_f {
                // login = tokens[3..8] (5 tokens: "Mon Feb  3 14:29:24 2021")
                let login_end = rest + 5;
                if login_end <= tokens.len() {
                    let login = tokens[rest..login_end].join(" ");
                    let ts = parse_timestamp(&login, None);
                    if let Some(e) = ts.naive_epoch {
                        obj.insert("login_epoch".to_string(), Value::Number(e.into()));
                    }
                    obj.insert("login".to_string(), Value::String(login));
                }

                // Python uses fixed index login_end+1 for logout (skips "-" or "still" at login_end).
                let after = login_end + 1;

                if after < tokens.len() {
                    let logout_tok = tokens[after].clone();
                    const IGNORED: &[&str] = &["down", "crash"];
                    if logout_tok == "still_logged_in" {
                        obj.insert(
                            "logout".to_string(),
                            Value::String("still logged in".to_string()),
                        );
                    } else if logout_tok == "gone_-_no_logout" || logout_tok == "gone" {
                        obj.insert(
                            "logout".to_string(),
                            Value::String("gone - no logout".to_string()),
                        );
                    } else if IGNORED.contains(&logout_tok.as_str()) {
                        obj.insert("logout".to_string(), Value::String(logout_tok.clone()));
                        // duration is right after the logout token (no placeholder insertion)
                        let dur_idx = after + 1;
                        if dur_idx < tokens.len() {
                            let dur = tokens[dur_idx].trim_matches(|c| c == '(' || c == ')');
                            let dur = normalize_duration(dur);
                            if !dur.is_empty() {
                                obj.insert("duration".to_string(), Value::String(dur));
                            }
                        }
                    } else if is_last_f_at(&tokens, after + 3) {
                        // Full -F logout date (5 tokens: "Mon Feb  3 14:29:24 2021")
                        let logout_end = after + 5;
                        if logout_end <= tokens.len() {
                            let logout = tokens[after..logout_end].join(" ");
                            let ts2 = parse_timestamp(&logout, None);
                            if let Some(e2) = ts2.naive_epoch {
                                obj.insert("logout_epoch".to_string(), Value::Number(e2.into()));
                                if let Some(le) = obj.get("login_epoch").and_then(|v| v.as_i64()) {
                                    obj.insert(
                                        "duration_seconds".to_string(),
                                        Value::Number((e2 - le).into()),
                                    );
                                }
                            }
                            obj.insert("logout".to_string(), Value::String(logout));
                        }
                        // duration
                        let dur_idx = after + 5;
                        if dur_idx < tokens.len() {
                            let dur = tokens[dur_idx].trim_matches(|c| c == '(' || c == ')');
                            let dur = normalize_duration(dur);
                            if !dur.is_empty() {
                                obj.insert("duration".to_string(), Value::String(dur));
                            }
                        }
                    } else {
                        // Single-token logout (e.g. "running" from "still running")
                        let logout_val = match logout_tok.as_str() {
                            "still_logged_in" => "still logged in".to_string(),
                            "gone_-_no_logout" | "gone" => "gone - no logout".to_string(),
                            other => other.to_string(),
                        };
                        obj.insert("logout".to_string(), Value::String(logout_val));
                    }
                }
            } else {
                // Normal format: login = tokens[3..7] (4 tokens: "Mon Feb  3 14:29")
                let login_end = (rest + 4).min(tokens.len());
                if rest < tokens.len() {
                    let login = tokens[rest..login_end].join(" ");
                    obj.insert("login".to_string(), Value::String(login));
                }

                // Python uses fixed index 8 (rest+5) for logout, skipping the potential
                // "-" separator or "still" at index 7 (rest+4).
                let logout_idx = rest + 5;

                if logout_idx < tokens.len() {
                    let logout_tok = tokens[logout_idx].clone();
                    let logout_val = match logout_tok.as_str() {
                        "still_logged_in" => "still logged in".to_string(),
                        "gone_-_no_logout" | "gone" => "gone - no logout".to_string(),
                        _ => logout_tok.clone(),
                    };
                    obj.insert("logout".to_string(), Value::String(logout_val));

                    // Duration is next token
                    let dur_idx = logout_idx + 1;
                    if dur_idx < tokens.len() {
                        let dur = tokens[dur_idx].trim_matches(|c| c == '(' || c == ')');
                        if !dur.is_empty() && dur != "-" {
                            let dur = normalize_duration(dur);
                            obj.insert("duration".to_string(), Value::String(dur));
                        }
                    }
                }
            }

            result.push(obj);
        }

        Ok(ParseOutput::Array(result))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_last_centos() {
        let input = include_str!("../../../../tests/fixtures/centos-7.7/last.out");
        let expected: Vec<serde_json::Value> = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/centos-7.7/last.json"
        ))
        .unwrap();
        let parser = LastParser;
        let result = parser.parse(input, false).unwrap();
        if let ParseOutput::Array(arr) = result {
            assert_eq!(arr.len(), expected.len(), "record count mismatch");
            for (i, (got, exp)) in arr.iter().zip(expected.iter()).enumerate() {
                assert_eq!(got["user"], exp["user"], "user mismatch at row {}", i);
                assert_eq!(got["tty"], exp["tty"], "tty mismatch at row {}", i);
                assert_eq!(
                    got["hostname"], exp["hostname"],
                    "hostname mismatch at row {}",
                    i
                );
                assert_eq!(got["login"], exp["login"], "login mismatch at row {}", i);
            }
        } else {
            panic!("Expected Array");
        }
    }

    #[test]
    fn test_last_empty() {
        let parser = LastParser;
        let result = parser.parse("", false).unwrap();
        if let ParseOutput::Array(arr) = result {
            assert!(arr.is_empty());
        } else {
            panic!("Expected Array");
        }
    }
}
