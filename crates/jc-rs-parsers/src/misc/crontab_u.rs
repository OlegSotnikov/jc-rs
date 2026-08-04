//! Parser for `crontab -l` output with user field support.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};

pub struct CrontabUParser;

static INFO: ParserInfo = ParserInfo {
    name: "crontab_u",
    argument: "--crontab-u",
    version: "1.10.0",
    description: "Converts `crontab` file with user field to JSON",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[
        Platform::Linux,
        Platform::Darwin,
        Platform::Aix,
        Platform::FreeBSD,
    ],
    tags: &[Tag::File, Tag::Command],
    magic_commands: &[],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static CRONTAB_U_PARSER: CrontabUParser = CrontabUParser;

inventory::submit! {
    ParserEntry::new(&CRONTAB_U_PARSER)
}

fn split_cron_field(s: &str) -> Vec<Value> {
    s.split(',').map(|p| Value::String(p.to_string())).collect()
}

impl Parser for CrontabUParser {
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

        let mut variables: Vec<Value> = Vec::new();
        let mut schedule: Vec<Value> = Vec::new();

        let mut cleandata: Vec<String> = input
            .lines()
            .map(|l| l.to_string())
            .filter(|l| !l.trim().is_empty())
            .collect();

        // Remove comment lines
        cleandata.retain(|l| !l.trim().starts_with('#'));

        // Extract variable assignment lines
        let mut remaining: Vec<String> = Vec::new();
        for line in &cleandata {
            let trimmed = line.trim();
            let first_char = trimmed.chars().next().unwrap_or(' ');
            if trimmed.contains('=')
                && !first_char.is_ascii_digit()
                && first_char != '@'
                && first_char != '*'
            {
                if let Some(eq_pos) = trimmed.find('=') {
                    let name = trimmed[..eq_pos].trim().to_string();
                    let value = trimmed[eq_pos + 1..].trim().to_string();
                    let mut var_obj = Map::new();
                    var_obj.insert("name".to_string(), Value::String(name));
                    var_obj.insert("value".to_string(), Value::String(value));
                    variables.push(Value::Object(var_obj));
                }
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

        // Parse normal cron lines first (with user field), then @shorthand at end
        // Format: minute hour day_of_month month day_of_week user command
        let mut shorthand_entries: Vec<Value> = Vec::new();
        for line in &remaining {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            if trimmed.starts_with('@') {
                // Format: @occurrence user command
                let tokens: Vec<&str> = trimmed.split_whitespace().collect();
                let occurrence = tokens.first().unwrap_or(&"").trim_start_matches('@');
                let user = tokens.get(1).copied().unwrap_or("").trim();
                // Rebuild command from remaining tokens
                let cmd = if tokens.len() > 2 {
                    tokens[2..].join(" ")
                } else {
                    String::new()
                };
                let mut entry = Map::new();
                entry.insert(
                    "occurrence".to_string(),
                    Value::String(occurrence.to_string()),
                );
                entry.insert("user".to_string(), Value::String(user.to_string()));
                entry.insert("command".to_string(), Value::String(cmd));
                shorthand_entries.push(Value::Object(entry));
            } else {
                // Split on whitespace runs, collect up to 7 tokens (last gets remainder)
                let mut tokens = trimmed.splitn(7, |c: char| c.is_whitespace());
                let t: Vec<&str> = {
                    let mut v = Vec::new();
                    for tok in &mut tokens {
                        if tok.is_empty() {
                            continue;
                        }
                        v.push(tok);
                    }
                    v
                };
                // Re-split properly using split_whitespace with limit
                let all_tokens: Vec<&str> = trimmed.split_whitespace().collect();
                if all_tokens.len() < 7 {
                    continue;
                }
                // Find command start position in original line to preserve internal spacing
                let cmd = {
                    let mut pos = 0usize;
                    let mut fields_skipped = 0;
                    let bytes = trimmed.as_bytes();
                    while fields_skipped < 6 && pos < bytes.len() {
                        // skip whitespace
                        while pos < bytes.len() && (bytes[pos] == b' ' || bytes[pos] == b'\t') {
                            pos += 1;
                        }
                        // skip non-whitespace (field)
                        while pos < bytes.len() && bytes[pos] != b' ' && bytes[pos] != b'\t' {
                            pos += 1;
                        }
                        fields_skipped += 1;
                    }
                    // skip whitespace before command
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
                entry.insert("user".to_string(), Value::String(all_tokens[5].to_string()));
                entry.insert("command".to_string(), Value::String(cmd));
                schedule.push(Value::Object(entry));
                let _ = t; // suppress unused warning
            }
        }
        // Append @shorthand entries after normal entries (matching jc behavior)
        schedule.extend(shorthand_entries);

        let mut obj = Map::new();
        obj.insert("variables".to_string(), Value::Array(variables));
        obj.insert("schedule".to_string(), Value::Array(schedule));
        Ok(ParseOutput::Object(obj))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crontab_u_centos() {
        let input = include_str!("../../../../tests/fixtures/centos-7.7/crontab-u.out");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/centos-7.7/crontab-u.json"
        ))
        .unwrap();
        let parser = CrontabUParser;
        let result = parser.parse(input, false).unwrap();
        if let ParseOutput::Object(obj) = result {
            let got = serde_json::Value::Object(obj);
            assert_eq!(
                got["variables"].as_array().map(|a| a.len()),
                expected["variables"].as_array().map(|a| a.len()),
                "variable count mismatch"
            );
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
    fn test_crontab_u_no_normal_entries() {
        let input =
            include_str!("../../../../tests/fixtures/generic/crontab-u-no-normal-entries.out");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/generic/crontab-u-no-normal-entries.json"
        ))
        .unwrap();
        let parser = CrontabUParser;
        let result = parser.parse(input, false).unwrap();
        if let ParseOutput::Object(obj) = result {
            let got = serde_json::Value::Object(obj);
            assert_eq!(
                got["schedule"][0]["occurrence"],
                expected["schedule"][0]["occurrence"]
            );
        } else {
            panic!("Expected Object");
        }
    }

    #[test]
    fn test_crontab_u_var_fix() {
        let input = include_str!("../../../../tests/fixtures/generic/crontab-u-var-fix.out");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/generic/crontab-u-var-fix.json"
        ))
        .unwrap();
        let parser = CrontabUParser;
        let result = parser.parse(input, false).unwrap();
        if let ParseOutput::Object(obj) = result {
            let got = serde_json::Value::Object(obj);
            assert_eq!(got["schedule"][0]["user"], expected["schedule"][0]["user"]);
            assert_eq!(
                got["schedule"][0]["command"],
                expected["schedule"][0]["command"]
            );
        } else {
            panic!("Expected Object");
        }
    }

    #[test]
    fn test_crontab_u_empty() {
        let parser = CrontabUParser;
        let result = parser.parse("", false).unwrap();
        if let ParseOutput::Object(obj) = result {
            assert!(obj["variables"].as_array().unwrap().is_empty());
            assert!(obj["schedule"].as_array().unwrap().is_empty());
        } else {
            panic!("Expected Object");
        }
    }
}
