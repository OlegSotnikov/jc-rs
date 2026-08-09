//! Parser for `crontab -l` output and crontab file format.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};

pub struct CrontabParser;

static INFO: ParserInfo = ParserInfo {
    name: "crontab",
    argument: "--crontab",
    version: "1.9.0",
    description: "Converts `crontab` command and file output to JSON",
    author: "jc-rs contributors",
    author_email: "os@g1sw.com",
    compatible: &[
        Platform::Linux,
        Platform::Darwin,
        Platform::Aix,
        Platform::FreeBSD,
    ],
    tags: &[Tag::File, Tag::Command],
    magic_commands: &["crontab"],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static CRONTAB_PARSER: CrontabParser = CrontabParser;

inventory::submit! {
    ParserEntry::new(&CRONTAB_PARSER)
}

/// Split a cron time field (comma-separated) into an array of strings.
fn split_cron_field(s: &str) -> Vec<Value> {
    s.split(',').map(|p| Value::String(p.to_string())).collect()
}

pub(crate) fn parse_crontab_input(input: &str) -> Map<String, Value> {
    let mut variables: Vec<Value> = Vec::new();
    let mut schedule: Vec<Value> = Vec::new();

    // Process lines: filter blank lines and comment lines
    let mut cleandata: Vec<String> = input
        .lines()
        .map(|l| l.to_string())
        .filter(|l| !l.trim().is_empty())
        .collect();

    // Remove comment lines
    cleandata.retain(|l| !l.trim().starts_with('#'));

    if cleandata.is_empty() {
        let mut obj = Map::new();
        obj.insert("variables".to_string(), Value::Array(vec![]));
        obj.insert("schedule".to_string(), Value::Array(vec![]));
        return obj;
    }

    // Extract variable assignment lines (key=value), but not lines starting
    // with digit, '@', or '*'
    let mut remaining: Vec<String> = Vec::new();
    for line in &cleandata {
        let trimmed = line.trim();
        let first_char = trimmed.chars().next().unwrap_or(' ');
        if trimmed.contains('=')
            && !first_char.is_ascii_digit()
            && first_char != '@'
            && first_char != '*'
        {
            // Variable assignment
            let (name, value) = if let Some(eq_pos) = trimmed.find('=') {
                let name = trimmed[..eq_pos].trim().to_string();
                let value = trimmed[eq_pos + 1..].trim().to_string();
                (name, value)
            } else {
                continue;
            };
            let mut var_obj = Map::new();
            var_obj.insert("name".to_string(), Value::String(name));
            var_obj.insert("value".to_string(), Value::String(value));
            variables.push(Value::Object(var_obj));
        } else {
            remaining.push(line.clone());
        }
    }

    // Sort variables alphabetically by name (matching jc behavior)
    variables.sort_by(|a, b| {
        let na = a.get("name").and_then(|v| v.as_str()).unwrap_or("");
        let nb = b.get("name").and_then(|v| v.as_str()).unwrap_or("");
        na.cmp(nb)
    });

    // Parse normal cron lines first, collect @shorthand entries separately
    let mut shorthand_entries: Vec<Value> = Vec::new();
    for line in &remaining {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if trimmed.starts_with('@') {
            // Format: @occurrence command
            let tokens: Vec<&str> = trimmed.split_whitespace().collect();
            let occurrence = tokens
                .first()
                .copied()
                .unwrap_or("")
                .trim_start_matches('@');
            // Command is everything after the occurrence token (preserve original spacing)
            let cmd = {
                let pos = trimmed.find(char::is_whitespace).unwrap_or(trimmed.len());
                let rest = trimmed[pos..].trim_start();
                rest.to_string()
            };
            let mut entry = Map::new();
            entry.insert(
                "occurrence".to_string(),
                Value::String(occurrence.to_string()),
            );
            entry.insert("command".to_string(), Value::String(cmd));
            shorthand_entries.push(Value::Object(entry));
        } else {
            // Format: minute hour day_of_month month day_of_week command
            let all_tokens: Vec<&str> = trimmed.split_whitespace().collect();
            if all_tokens.len() < 6 {
                continue;
            }
            // Find command start in original line to preserve internal spacing
            let cmd = {
                let mut pos = 0usize;
                let bytes = trimmed.as_bytes();
                for _ in 0..5 {
                    while pos < bytes.len() && (bytes[pos] == b' ' || bytes[pos] == b'\t') {
                        pos += 1;
                    }
                    while pos < bytes.len() && bytes[pos] != b' ' && bytes[pos] != b'\t' {
                        pos += 1;
                    }
                }
                while pos < bytes.len() && (bytes[pos] == b' ' || bytes[pos] == b'\t') {
                    pos += 1;
                }
                trimmed[pos..].to_string()
            };

            let mut entry = Map::new();
            entry.insert(
                "minute".to_string(),
                Value::Array(split_cron_field(all_tokens[0])),
            );
            entry.insert(
                "hour".to_string(),
                Value::Array(split_cron_field(all_tokens[1])),
            );
            entry.insert(
                "day_of_month".to_string(),
                Value::Array(split_cron_field(all_tokens[2])),
            );
            entry.insert(
                "month".to_string(),
                Value::Array(split_cron_field(all_tokens[3])),
            );
            entry.insert(
                "day_of_week".to_string(),
                Value::Array(split_cron_field(all_tokens[4])),
            );
            entry.insert("command".to_string(), Value::String(cmd));
            schedule.push(Value::Object(entry));
        }
    }
    // Append @shorthand entries after normal entries, in reverse file order (matching jc behavior)
    shorthand_entries.reverse();
    schedule.extend(shorthand_entries);

    let mut obj = Map::new();
    obj.insert("variables".to_string(), Value::Array(variables));
    obj.insert("schedule".to_string(), Value::Array(schedule));
    obj
}

impl Parser for CrontabParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            let mut obj = Map::new();
            obj.insert("variables".to_string(), Value::Array(vec![]));
            obj.insert("schedule".to_string(), Value::Array(vec![]));
            return Ok(ParseOutput::Object(obj));
        }

        let obj = parse_crontab_input(input);
        Ok(ParseOutput::Object(obj))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crontab_centos() {
        let input = include_str!("../../../../tests/fixtures/centos-7.7/crontab.out");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/centos-7.7/crontab.json"
        ))
        .unwrap();
        let parser = CrontabParser;
        let result = parser.parse(input, false).unwrap();
        if let ParseOutput::Object(obj) = result {
            let got = serde_json::Value::Object(obj);
            // Check variables
            assert_eq!(
                got["variables"].as_array().map(|a| a.len()),
                expected["variables"].as_array().map(|a| a.len()),
                "variable count mismatch"
            );
            // Check schedule count
            assert_eq!(
                got["schedule"].as_array().map(|a| a.len()),
                expected["schedule"].as_array().map(|a| a.len()),
                "schedule count mismatch"
            );
        } else {
            panic!("Expected Object");
        }
    }

    #[test]
    fn test_crontab_no_normal_entries() {
        let input =
            include_str!("../../../../tests/fixtures/generic/crontab-no-normal-entries.out");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/generic/crontab-no-normal-entries.json"
        ))
        .unwrap();
        let parser = CrontabParser;
        let result = parser.parse(input, false).unwrap();
        if let ParseOutput::Object(obj) = result {
            let got = serde_json::Value::Object(obj);
            assert_eq!(
                got["schedule"][0]["occurrence"],
                expected["schedule"][0]["occurrence"]
            );
            assert_eq!(
                got["schedule"][0]["command"],
                expected["schedule"][0]["command"]
            );
        } else {
            panic!("Expected Object");
        }
    }

    #[test]
    fn test_crontab_var_fix() {
        let input = include_str!("../../../../tests/fixtures/generic/crontab-var-fix.out");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/generic/crontab-var-fix.json"
        ))
        .unwrap();
        let parser = CrontabParser;
        let result = parser.parse(input, false).unwrap();
        if let ParseOutput::Object(obj) = result {
            let got = serde_json::Value::Object(obj);
            assert_eq!(
                got["schedule"][0]["command"],
                expected["schedule"][0]["command"]
            );
        } else {
            panic!("Expected Object");
        }
    }

    #[test]
    fn test_crontab_empty() {
        let parser = CrontabParser;
        let result = parser.parse("", false).unwrap();
        if let ParseOutput::Object(obj) = result {
            assert!(obj["variables"].as_array().unwrap().is_empty());
            assert!(obj["schedule"].as_array().unwrap().is_empty());
        } else {
            panic!("Expected Object");
        }
    }
}
