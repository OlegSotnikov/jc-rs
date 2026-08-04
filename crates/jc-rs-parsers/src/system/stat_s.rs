//! Streaming parser for `stat` command output.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use jc_rs_utils::{convert_to_int, parse_timestamp};
use serde_json::{Map, Value};

pub struct StatSParser;

static INFO: ParserInfo = ParserInfo {
    name: "stat_s",
    argument: "--stat-s",
    version: "1.13.0",
    description: "Streaming parser for `stat` command output",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[Platform::Linux, Platform::Darwin, Platform::FreeBSD],
    tags: &[Tag::Command, Tag::Streaming],
    magic_commands: &[],
    streaming: true,
    hidden: false,
    deprecated: false,
};

static STAT_S_PARSER: StatSParser = StatSParser;

inventory::submit! {
    ParserEntry::new(&STAT_S_PARSER)
}

fn parse_time_field(val: &str, obj: &mut Map<String, Value>, key: &str) {
    if val == "-" {
        obj.insert(key.to_string(), Value::Null);
        obj.insert(format!("{}_epoch", key), Value::Null);
        obj.insert(format!("{}_epoch_utc", key), Value::Null);
    } else {
        obj.insert(key.to_string(), Value::String(val.to_string()));
        let ts = parse_timestamp(val, None);
        obj.insert(
            format!("{}_epoch", key),
            ts.naive_epoch.map(Value::from).unwrap_or(Value::Null),
        );
        obj.insert(
            format!("{}_epoch_utc", key),
            ts.utc_epoch.map(Value::from).unwrap_or(Value::Null),
        );
    }
}

fn shell_split(s: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;

    for ch in s.chars() {
        match ch {
            '"' => {
                in_quotes = !in_quotes;
            }
            ' ' | '\t' if !in_quotes => {
                if !current.is_empty() {
                    result.push(current.clone());
                    current.clear();
                }
            }
            _ => {
                current.push(ch);
            }
        }
    }
    if !current.is_empty() {
        result.push(current);
    }
    result
}

/// stat_s delegates to the same logic as stat but wraps each record as a streaming item.
impl Parser for StatSParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Array(vec![]));
        }

        let cleandata: Vec<&str> = input.lines().filter(|l| !l.trim().is_empty()).collect();
        if cleandata.is_empty() {
            return Ok(ParseOutput::Array(vec![]));
        }

        let result = if cleandata[0].find("File:") == Some(2) {
            parse_linux_streaming(&cleandata)
        } else {
            parse_bsd_streaming(&cleandata)
        };

        Ok(ParseOutput::Array(result))
    }
}

fn parse_linux_streaming(cleandata: &[&str]) -> Vec<Map<String, Value>> {
    let mut result = Vec::new();
    let mut obj: Map<String, Value> = Map::new();

    for line in cleandata {
        if line.find("File:") == Some(2) {
            if !obj.is_empty() {
                result.push(obj.clone());
                obj = Map::new();
            }
            let after = line.splitn(2, "File:").nth(1).unwrap_or("").trim();
            let after = after
                .trim_matches('\'')
                .trim_matches('\u{2018}')
                .trim_matches('\u{2019}');
            if let Some(arrow_pos) = after.find(" -> ") {
                let filename = after[..arrow_pos].trim_matches('\'');
                let link_to = after[arrow_pos + 4..].trim_matches('\'');
                obj.insert("file".to_string(), Value::String(filename.to_string()));
                obj.insert("link_to".to_string(), Value::String(link_to.to_string()));
            } else {
                obj.insert("file".to_string(), Value::String(after.to_string()));
            }
            continue;
        }
        if line.starts_with("  Size:") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 8 {
                obj.insert(
                    "size".to_string(),
                    convert_to_int(parts[1])
                        .map(Value::from)
                        .unwrap_or(Value::Null),
                );
                obj.insert(
                    "blocks".to_string(),
                    convert_to_int(parts[3])
                        .map(Value::from)
                        .unwrap_or(Value::Null),
                );
                obj.insert(
                    "io_blocks".to_string(),
                    convert_to_int(parts[6])
                        .map(Value::from)
                        .unwrap_or(Value::Null),
                );
                obj.insert("type".to_string(), Value::String(parts[7..].join(" ")));
            }
            continue;
        }
        if line.starts_with("Device:") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 6 {
                obj.insert("device".to_string(), Value::String(parts[1].to_string()));
                obj.insert(
                    "inode".to_string(),
                    convert_to_int(parts[3])
                        .map(Value::from)
                        .unwrap_or(Value::Null),
                );
                obj.insert(
                    "links".to_string(),
                    convert_to_int(parts[5])
                        .map(Value::from)
                        .unwrap_or(Value::Null),
                );
            }
            continue;
        }
        if line.starts_with("Access: (") {
            let cleaned = line.replace('(', " ").replace(')', " ").replace('/', " ");
            let parts: Vec<&str> = cleaned.split_whitespace().collect();
            if parts.len() >= 9 {
                obj.insert("access".to_string(), Value::String(parts[1].to_string()));
                obj.insert("flags".to_string(), Value::String(parts[2].to_string()));
                obj.insert(
                    "uid".to_string(),
                    convert_to_int(parts[4])
                        .map(Value::from)
                        .unwrap_or(Value::Null),
                );
                obj.insert("user".to_string(), Value::String(parts[5].to_string()));
                obj.insert(
                    "gid".to_string(),
                    convert_to_int(parts[7])
                        .map(Value::from)
                        .unwrap_or(Value::Null),
                );
                obj.insert("group".to_string(), Value::String(parts[8].to_string()));
            }
            continue;
        }
        if line.starts_with("Access: 2")
            || line.starts_with("Access: 1")
            || line.starts_with("Access: -")
        {
            let after = line.splitn(2, "Access: ").nth(1).unwrap_or("").trim();
            parse_time_field(after, &mut obj, "access_time");
            continue;
        }
        if line.starts_with("Modify:") {
            let after = line.splitn(2, "Modify: ").nth(1).unwrap_or("").trim();
            parse_time_field(after, &mut obj, "modify_time");
            continue;
        }
        if line.starts_with("Change:") {
            let after = line.splitn(2, "Change: ").nth(1).unwrap_or("").trim();
            parse_time_field(after, &mut obj, "change_time");
            continue;
        }
        if line.starts_with(" Birth:") {
            let after = line.splitn(2, "Birth: ").nth(1).unwrap_or("").trim();
            parse_time_field(after, &mut obj, "birth_time");
            continue;
        }
    }

    if !obj.is_empty() {
        result.push(obj);
    }

    result
}

fn parse_bsd_streaming(cleandata: &[&str]) -> Vec<Map<String, Value>> {
    let mut result = Vec::new();
    for line in cleandata {
        if line.trim().is_empty() {
            continue;
        }
        let parts = shell_split(line);
        if parts.len() < 16 {
            continue;
        }
        let mut obj = Map::new();
        let filename = parts[15..].join(" ");
        obj.insert("file".to_string(), Value::String(filename));
        obj.insert(
            "unix_device".to_string(),
            convert_to_int(&parts[0])
                .map(Value::from)
                .unwrap_or(Value::Null),
        );
        obj.insert(
            "inode".to_string(),
            convert_to_int(&parts[1])
                .map(Value::from)
                .unwrap_or(Value::Null),
        );
        obj.insert("flags".to_string(), Value::String(parts[2].clone()));
        obj.insert(
            "links".to_string(),
            convert_to_int(&parts[3])
                .map(Value::from)
                .unwrap_or(Value::Null),
        );
        obj.insert("user".to_string(), Value::String(parts[4].clone()));
        obj.insert("group".to_string(), Value::String(parts[5].clone()));
        obj.insert(
            "rdev".to_string(),
            convert_to_int(&parts[6])
                .map(Value::from)
                .unwrap_or(Value::Null),
        );
        obj.insert(
            "size".to_string(),
            convert_to_int(&parts[7])
                .map(Value::from)
                .unwrap_or(Value::Null),
        );
        parse_time_field(&parts[8], &mut obj, "access_time");
        parse_time_field(&parts[9], &mut obj, "modify_time");
        parse_time_field(&parts[10], &mut obj, "change_time");
        parse_time_field(&parts[11], &mut obj, "birth_time");
        obj.insert(
            "block_size".to_string(),
            convert_to_int(&parts[12])
                .map(Value::from)
                .unwrap_or(Value::Null),
        );
        obj.insert(
            "blocks".to_string(),
            convert_to_int(&parts[13])
                .map(Value::from)
                .unwrap_or(Value::Null),
        );
        obj.insert("unix_flags".to_string(), Value::String(parts[14].clone()));
        result.push(obj);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stat_s_centos() {
        let input = include_str!("../../../../tests/fixtures/centos-7.7/stat.out");
        let expected: Vec<serde_json::Value> = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/centos-7.7/stat-streaming.json"
        ))
        .unwrap();
        let parser = StatSParser;
        let result = parser.parse(input, false).unwrap();
        if let ParseOutput::Array(arr) = result {
            assert_eq!(arr.len(), expected.len(), "record count mismatch");
        } else {
            panic!("Expected Array");
        }
    }

    #[test]
    fn test_stat_s_empty() {
        let parser = StatSParser;
        let result = parser.parse("", false).unwrap();
        if let ParseOutput::Array(arr) = result {
            assert!(arr.is_empty());
        } else {
            panic!("Expected Array");
        }
    }
}
