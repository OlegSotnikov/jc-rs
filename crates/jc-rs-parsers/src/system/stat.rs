//! Parser for `stat` command output.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use jc_rs_utils::{convert_to_int, parse_timestamp, timestamp::formats};
use serde_json::{Map, Value};

pub struct StatParser;

static INFO: ParserInfo = ParserInfo {
    name: "stat",
    argument: "--stat",
    version: "1.13.0",
    description: "Converts `stat` command output to JSON",
    author: "jc-rs contributors",
    author_email: "os@g1sw.com",
    compatible: &[Platform::Linux, Platform::Darwin, Platform::FreeBSD],
    tags: &[Tag::Command],
    magic_commands: &["stat"],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static STAT_PARSER: StatParser = StatParser;

inventory::submit! {
    ParserEntry::new(&STAT_PARSER)
}

fn parse_time_field(val: &str, obj: &mut Map<String, Value>, key: &str) {
    if val == "-" {
        obj.insert(key.to_string(), Value::Null);
        obj.insert(format!("{}_epoch", key), Value::Null);
        obj.insert(format!("{}_epoch_utc", key), Value::Null);
    } else {
        obj.insert(key.to_string(), Value::String(val.to_string()));
        let ts = parse_timestamp(val, &[formats::F7100, formats::F7200]);
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

/// Parse Linux `stat` format (multi-line per file starting with "  File: ...")
fn parse_linux(cleandata: &[&str]) -> Vec<Map<String, Value>> {
    let mut result = Vec::new();
    let mut obj: Map<String, Value> = Map::new();

    for line in cleandata {
        // Line 1: "  File: '/bin/['"
        if line.find("File:") == Some(2) {
            if !obj.is_empty() {
                result.push(obj.clone());
                obj = Map::new();
            }

            let after = line.split_once("File:").map(|x| x.1).unwrap_or("").trim();
            // Handle symlinks: "'/bin/apropos' -> 'whatis'"
            let after = after
                .trim_matches('\'')
                .trim_matches('\u{2018}')
                .trim_matches('\u{2019}');
            if let Some(arrow_pos) = after.find(" -> ") {
                let filename = after[..arrow_pos]
                    .trim_matches('\'')
                    .trim_matches('\u{2018}')
                    .trim_matches('\u{2019}');
                let link_to = after[arrow_pos + 4..]
                    .trim_matches('\'')
                    .trim_matches('\u{2018}')
                    .trim_matches('\u{2019}');
                obj.insert("file".to_string(), Value::String(filename.to_string()));
                obj.insert("link_to".to_string(), Value::String(link_to.to_string()));
            } else {
                obj.insert("file".to_string(), Value::String(after.to_string()));
            }
            continue;
        }

        // Line 2: "  Size: 41488     Blocks: 88         IO Block: 4096   regular file"
        if line.starts_with("  Size:") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            // parts[0]=Size: parts[1]=val parts[2]=Blocks: parts[3]=val parts[4]=IO parts[5]=Block: parts[6]=val parts[7..]=type
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

        // Line 3: "Device: fd00h/64768d    Inode: 50332811    Links: 1"
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

        // Line 4: "Access: (0755/-rwxr-xr-x)  Uid: (    0/    root)   Gid: (    0/    root)"
        if line.starts_with("Access: (") {
            let cleaned = line.replace(['(', ')', '/'], " ");
            let parts: Vec<&str> = cleaned.split_whitespace().collect();
            // After replace: "Access:  0755  -rwxr-xr-x   Uid:      0     root    Gid:      0     root"
            // indices:         0       1      2            3        4      5        6        7      8
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

        // Access time line: "Access: 2019-08-19 23:25:31.000000000 -0700"
        if line.starts_with("Access: 2")
            || line.starts_with("Access: 1")
            || line.starts_with("Access: -")
        {
            let after = line
                .split_once("Access: ")
                .map(|x| x.1)
                .unwrap_or("")
                .trim();
            parse_time_field(after, &mut obj, "access_time");
            continue;
        }

        // Modify time
        if line.starts_with("Modify:") {
            let after = line
                .split_once("Modify: ")
                .map(|x| x.1)
                .unwrap_or("")
                .trim();
            parse_time_field(after, &mut obj, "modify_time");
            continue;
        }

        // Change time
        if line.starts_with("Change:") {
            let after = line
                .split_once("Change: ")
                .map(|x| x.1)
                .unwrap_or("")
                .trim();
            parse_time_field(after, &mut obj, "change_time");
            continue;
        }

        // Birth time
        if line.starts_with(" Birth:") {
            let after = line.split_once("Birth: ").map(|x| x.1).unwrap_or("").trim();
            parse_time_field(after, &mut obj, "birth_time");
            continue;
        }
    }

    if !obj.is_empty() {
        result.push(obj);
    }

    result
}

/// Parse macOS/FreeBSD `stat` format (one line per file)
/// Format: unix_device inode flags links user group rdev size "atime" "mtime" "ctime" "btime" block_size blocks unix_flags filename...
fn parse_bsd(cleandata: &[&str]) -> Vec<Map<String, Value>> {
    let mut result = Vec::new();

    for line in cleandata {
        if line.trim().is_empty() {
            continue;
        }

        // Use shell-like splitting to handle quoted timestamps
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

/// Simple shell-like split that handles double-quoted tokens
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

impl Parser for StatParser {
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

        // Detect format: Linux starts with "  File: ..."
        let result = if cleandata[0].find("File:") == Some(2) {
            parse_linux(&cleandata)
        } else {
            parse_bsd(&cleandata)
        };

        Ok(ParseOutput::Array(result))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stat_centos() {
        let input = include_str!("../../../../tests/fixtures/centos-7.7/stat.out");
        let expected: Vec<serde_json::Value> = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/centos-7.7/stat.json"
        ))
        .unwrap();
        let parser = StatParser;
        let result = parser.parse(input, false).unwrap();
        if let ParseOutput::Array(arr) = result {
            assert_eq!(arr.len(), expected.len(), "record count mismatch");
            for (i, (got, exp)) in arr.iter().zip(expected.iter()).enumerate() {
                assert_eq!(got["file"], exp["file"], "file mismatch at row {}", i);
                assert_eq!(got["size"], exp["size"], "size mismatch at row {}", i);
                assert_eq!(got["inode"], exp["inode"], "inode mismatch at row {}", i);
            }
        } else {
            panic!("Expected Array");
        }
    }

    #[test]
    fn test_stat_osx() {
        let input = include_str!("../../../../tests/fixtures/osx-10.14.6/stat.out");
        let expected: Vec<serde_json::Value> = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/osx-10.14.6/stat.json"
        ))
        .unwrap();
        let parser = StatParser;
        let result = parser.parse(input, false).unwrap();
        if let ParseOutput::Array(arr) = result {
            assert_eq!(arr.len(), expected.len(), "osx record count mismatch");
            for (i, (got, exp)) in arr.iter().zip(expected.iter()).enumerate() {
                assert_eq!(got["file"], exp["file"], "file mismatch at row {}", i);
                assert_eq!(got["size"], exp["size"], "size mismatch at row {}", i);
            }
        } else {
            panic!("Expected Array");
        }
    }

    #[test]
    fn test_stat_empty() {
        let parser = StatParser;
        let result = parser.parse("", false).unwrap();
        if let ParseOutput::Array(arr) = result {
            assert!(arr.is_empty());
        } else {
            panic!("Expected Array");
        }
    }
}
