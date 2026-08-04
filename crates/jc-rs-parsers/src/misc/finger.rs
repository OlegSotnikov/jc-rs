//! Parser for `finger` command output.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use jc_rs_utils::sparse_table_parse;
use regex::Regex;
use serde_json::{Map, Value};

pub struct FingerParser;

static INFO: ParserInfo = ParserInfo {
    name: "finger",
    argument: "--finger",
    version: "1.2.0",
    description: "`finger` command parser",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[Platform::Linux, Platform::Darwin, Platform::FreeBSD],
    tags: &[Tag::Command],
    magic_commands: &["finger"],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static FINGER_PARSER: FingerParser = FingerParser;

inventory::submit! {
    ParserEntry::new(&FINGER_PARSER)
}

impl Parser for FingerParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Array(vec![]));
        }

        // Filter empty lines
        let data_lines: Vec<&str> = input.lines().filter(|l| !l.is_empty()).collect();
        if data_lines.is_empty() {
            return Ok(ParseOutput::Array(vec![]));
        }

        // Find the column split position: end of "Idle" in the header
        let header = data_lines[0];
        let sep_col = match header.find("Idle") {
            Some(pos) => pos + 4,
            None => return Ok(ParseOutput::Array(vec![])),
        };

        // Split each line into first_half (table data) and second_half (login time + details)
        let mut first_halves: Vec<String> = Vec::new();
        let mut second_halves: Vec<String> = Vec::new();

        for line in &data_lines {
            let line_bytes = line.len();
            if line_bytes >= sep_col {
                first_halves.push(line[..sep_col].to_string());
                second_halves.push(line[sep_col..].to_string());
            } else {
                first_halves.push(line.to_string());
                second_halves.push(String::new());
            }
        }

        // Lowercase the header
        if let Some(h) = first_halves.first_mut() {
            *h = h.to_lowercase();
        }

        // Parse the first half table
        let table_input = first_halves.join("\n");
        let raw_rows = sparse_table_parse(&table_input);

        // Regex to extract login time and optional details from second half
        // Pattern: month_abbr whitespace day_num whitespace (HH:MM|YYYY) optional_rest
        let login_re = Regex::new(r"([A-Z][a-z]{2}\s+\d{1,2}\s+)(\d\d:\d\d|\d{4})(\s?.+)?$")
            .map_err(|e| ParseError::Generic(e.to_string()))?;

        // Process each data row (skip header row from second_halves)
        let second_halves_data = &second_halves[1..];

        let mut result = Vec::new();

        for (i, row) in raw_rows.iter().enumerate() {
            let mut obj = Map::new();

            // login
            if let Some(v) = row.get("login") {
                obj.insert("login".to_string(), v.clone());
            }

            // name
            if let Some(v) = row.get("name") {
                obj.insert("name".to_string(), v.clone());
            }

            // tty: detect writeable from '*' prefix
            let tty_raw = row.get("tty").and_then(|v| match v {
                Value::String(s) => Some(s.clone()),
                _ => None,
            });
            let (tty_clean, tty_writeable) = match tty_raw.as_deref() {
                Some(s) if s.contains('*') => (s.replace('*', ""), false),
                Some(s) => (s.to_string(), true),
                None => (String::new(), true),
            };
            obj.insert("tty".to_string(), Value::String(tty_clean));

            // idle: '-' → null, "" → null
            let idle_raw = row.get("idle").and_then(|v| match v {
                Value::String(s) => Some(s.clone()),
                Value::Null => None,
                _ => None,
            });
            let idle = match idle_raw.as_deref() {
                Some("-") | None | Some("") => None,
                Some(s) => Some(s.to_string()),
            };
            obj.insert(
                "idle".to_string(),
                idle.as_ref()
                    .map(|s| Value::String(s.clone()))
                    .unwrap_or(Value::Null),
            );

            // login_time and details from second half
            let second = second_halves_data.get(i).map(|s| s.as_str()).unwrap_or("");
            if let Some(caps) = login_re.captures(second) {
                let date_part = caps.get(1).map_or("", |m| m.as_str()).trim();
                let time_part = caps.get(2).map_or("", |m| m.as_str()).trim();
                let login_time = format!("{} {}", date_part, time_part);
                obj.insert("login_time".to_string(), Value::String(login_time));

                if let Some(details_match) = caps.get(3) {
                    let details = details_match.as_str().trim();
                    if !details.is_empty() {
                        obj.insert("details".to_string(), Value::String(details.to_string()));
                    }
                }
            }

            // Computed idle fields
            let mut idle_minutes = 0i64;
            let mut idle_hours = 0i64;
            let mut idle_days = 0i64;

            if let Some(ref idle_str) = idle {
                if idle_str.chars().all(|c| c.is_ascii_digit()) {
                    idle_minutes = idle_str.parse().unwrap_or(0);
                } else if idle_str.contains(':') {
                    let parts: Vec<&str> = idle_str.splitn(2, ':').collect();
                    idle_hours = parts[0].parse().unwrap_or(0);
                    idle_minutes = parts[1].parse().unwrap_or(0);
                } else if idle_str.contains('d') {
                    idle_days = idle_str.trim_end_matches('d').parse().unwrap_or(0);
                }
            }

            let total_idle_minutes = (idle_days * 1440) + (idle_hours * 60) + idle_minutes;

            obj.insert("tty_writeable".to_string(), Value::Bool(tty_writeable));
            obj.insert("idle_minutes".to_string(), Value::from(idle_minutes));
            obj.insert("idle_hours".to_string(), Value::from(idle_hours));
            obj.insert("idle_days".to_string(), Value::from(idle_days));
            obj.insert(
                "total_idle_minutes".to_string(),
                Value::from(total_idle_minutes),
            );

            result.push(obj);
        }

        Ok(ParseOutput::Array(result))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_fixture(input: &str, expected_json: &str) {
        let parser = FingerParser;
        let result = parser.parse(input, false).unwrap();
        let expected: Vec<serde_json::Value> = serde_json::from_str(expected_json).unwrap();

        if let ParseOutput::Array(arr) = result {
            assert_eq!(arr.len(), expected.len(), "record count mismatch");
            for (i, (got, exp)) in arr.iter().zip(expected.iter()).enumerate() {
                assert_eq!(
                    serde_json::Value::Object(got.clone()),
                    *exp,
                    "mismatch at row {}",
                    i
                );
            }
        } else {
            panic!("Expected Array");
        }
    }

    #[test]
    fn test_finger_ubuntu() {
        parse_fixture(
            include_str!("../../../../tests/fixtures/ubuntu-18.04/finger.out"),
            include_str!("../../../../tests/fixtures/ubuntu-18.04/finger.json"),
        );
    }

    #[test]
    fn test_finger_osx() {
        parse_fixture(
            include_str!("../../../../tests/fixtures/osx-10.14.6/finger.out"),
            include_str!("../../../../tests/fixtures/osx-10.14.6/finger.json"),
        );
    }

    #[test]
    fn test_finger_empty() {
        let parser = FingerParser;
        let result = parser.parse("", false).unwrap();
        if let ParseOutput::Array(arr) = result {
            assert!(arr.is_empty());
        } else {
            panic!("Expected Array");
        }
    }
}
