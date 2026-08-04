//! Streaming parser for `rsync` command output.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use regex::Regex;
use serde_json::{Map, Value};

pub struct RsyncStreamParser;

static INFO: ParserInfo = ParserInfo {
    name: "rsync_s",
    argument: "--rsync-s",
    version: "1.3.0",
    description: "Streaming parser for `rsync` command output",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[Platform::Linux, Platform::Darwin, Platform::FreeBSD],
    tags: &[Tag::Command],
    streaming: true,
    hidden: false,
    deprecated: false,
    magic_commands: &[],
};

static RSYNC_STREAM_PARSER: RsyncStreamParser = RsyncStreamParser;
inventory::submit! { ParserEntry::new(&RSYNC_STREAM_PARSER) }

fn parse_size_to_int(s: &str) -> Option<i64> {
    let s = s.replace(',', "");
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    let (num, mult) = if let Some(n) = s.strip_suffix('K') {
        (n, 1024i64)
    } else if let Some(n) = s.strip_suffix('M') {
        (n, 1024 * 1024)
    } else if let Some(n) = s.strip_suffix('G') {
        (n, 1024 * 1024 * 1024)
    } else if let Some(n) = s.strip_suffix('T') {
        (n, 1024i64 * 1024 * 1024 * 1024)
    } else {
        (s, 1)
    };
    num.parse::<f64>().ok().map(|f| (f * mult as f64) as i64)
}

fn parse_size_to_float(s: &str) -> Option<f64> {
    let s = s.replace(',', "");
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    let (num, mult) = if let Some(n) = s.strip_suffix('K') {
        (n, 1024.0f64)
    } else if let Some(n) = s.strip_suffix('M') {
        (n, 1024.0 * 1024.0)
    } else if let Some(n) = s.strip_suffix('G') {
        (n, 1024.0 * 1024.0 * 1024.0)
    } else if let Some(n) = s.strip_suffix('T') {
        (n, 1024.0 * 1024.0 * 1024.0 * 1024.0)
    } else {
        (s, 1.0)
    };
    num.parse::<f64>().ok().map(|f| f * mult)
}

fn flag_bool(c: char, true_char: char) -> Value {
    match c {
        _ if c == true_char => Value::Bool(true),
        '.' => Value::Bool(false),
        '+' | ' ' | '?' => Value::Null,
        _ => Value::Null,
    }
}

fn parse_file_meta(meta: &str, name: &str) -> Map<String, Value> {
    let update_type_map: &[(&str, Option<&str>)] = &[
        ("<", Some("file sent")),
        (">", Some("file received")),
        ("c", Some("local change or creation")),
        ("h", Some("hard link")),
        (".", Some("not updated")),
        ("*", Some("message")),
        ("+", None),
    ];

    let file_type_map: &[(&str, Option<&str>)] = &[
        ("f", Some("file")),
        ("d", Some("directory")),
        ("L", Some("symlink")),
        ("D", Some("device")),
        ("S", Some("special file")),
        ("+", None),
    ];

    let chars: Vec<char> = meta.chars().collect();
    let get = |i: usize| chars.get(i).copied().unwrap_or('+');

    let update_type = update_type_map
        .iter()
        .find(|(k, _)| meta.starts_with(k))
        .map(|(_, v)| {
            v.map(|s| Value::String(s.to_string()))
                .unwrap_or(Value::Null)
        })
        .unwrap_or(Value::Null);

    let file_type = {
        let ft_char = get(1);
        file_type_map
            .iter()
            .find(|(k, _)| k.starts_with(ft_char))
            .map(|(_, v)| {
                v.map(|s| Value::String(s.to_string()))
                    .unwrap_or(Value::Null)
            })
            .unwrap_or(Value::Null)
    };

    let is_mac_format = meta.len() == 9;

    let mut obj = Map::new();
    obj.insert("type".to_string(), Value::String("file".to_string()));
    obj.insert("filename".to_string(), Value::String(name.to_string()));
    obj.insert("metadata".to_string(), Value::String(meta.to_string()));
    obj.insert("update_type".to_string(), update_type);
    obj.insert("file_type".to_string(), file_type);
    obj.insert(
        "checksum_or_value_different".to_string(),
        flag_bool(get(2), 'c'),
    );
    obj.insert("size_different".to_string(), flag_bool(get(3), 's'));
    obj.insert(
        "modification_time_different".to_string(),
        flag_bool(get(4), 't'),
    );
    obj.insert("permissions_different".to_string(), flag_bool(get(5), 'p'));
    obj.insert("owner_different".to_string(), flag_bool(get(6), 'o'));
    obj.insert("group_different".to_string(), flag_bool(get(7), 'g'));

    if !is_mac_format {
        obj.insert("acl_different".to_string(), flag_bool(get(9), 'a'));
        obj.insert(
            "extended_attribute_different".to_string(),
            flag_bool(get(10), 'x'),
        );
    }

    obj
}

impl Parser for RsyncStreamParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Array(vec![]));
        }

        let file_line_re = Regex::new(r"^([<>ch.*][fdlDS][c.+ ?][s.+ ?][t.+ ?][p.+ ?][o.+ ?][g.+ ?][u.+ ?][a.+ ?][x.+ ?]) (.+)$")
            .map_err(|e| ParseError::Regex(e.to_string()))?;
        let file_line_mac_re = Regex::new(
            r"^([<>ch.*][fdlDS][c.+ ?][s.+ ?][t.+ ?][p.+ ?][o.+ ?][g.+ ?][x.+ ?]) (.+)$",
        )
        .map_err(|e| ParseError::Regex(e.to_string()))?;
        let file_line_log_re = Regex::new(r"^(\d{4}/\d{2}/\d{2}) (\d{2}:\d{2}:\d{2}) \[(\d+)\] ([<>ch.*][fdlDS][c.+ ?][s.+ ?][t.+ ?][p.+ ?][o.+ ?][g.+ ?][u.+ ?][a.+ ?][x.+ ?]) (.+)$")
            .map_err(|e| ParseError::Regex(e.to_string()))?;
        let file_line_log_mac_re = Regex::new(r"^(\d{4}/\d{2}/\d{2}) (\d{2}:\d{2}:\d{2}) \[(\d+)\] ([<>ch.*][fdlDS][c.+ ?][s.+ ?][t.+ ?][p.+ ?][o.+ ?][g.+ ?][x.+ ?]) (.+)$")
            .map_err(|e| ParseError::Regex(e.to_string()))?;

        let stat1_re = Regex::new(
            r"sent\s+([0-9,]+)\s+bytes\s+received\s+([0-9,]+)\s+bytes\s+([0-9,.]+)\s+bytes/sec",
        )
        .map_err(|e| ParseError::Regex(e.to_string()))?;
        let stat2_re = Regex::new(r"total size is\s+([0-9,]+)\s+speedup is\s+([0-9,.]+)")
            .map_err(|e| ParseError::Regex(e.to_string()))?;
        let stat1_simple_re = Regex::new(r"sent\s+([0-9,.TGMK]+)\s+bytes\s+received\s+([0-9,.TGMK]+)\s+bytes\s+([0-9,.TGMK]+)\s+bytes/sec")
            .map_err(|e| ParseError::Regex(e.to_string()))?;
        let stat2_simple_re =
            Regex::new(r"total\s+size\s+is\s+([0-9,.TGMK]+)\s+speedup\s+is\s+([0-9,.TGMK]+)")
                .map_err(|e| ParseError::Regex(e.to_string()))?;
        let stat_log_re = Regex::new(r"^(\d{4}/\d{2}/\d{2}) (\d{2}:\d{2}:\d{2}) \[(\d+)\] sent\s+([\d,]+)\s+bytes\s+received\s+([\d,]+)\s+bytes\s+total\s+size\s+([\d,]+)")
            .map_err(|e| ParseError::Regex(e.to_string()))?;
        let stat1_log_v_re = Regex::new(r"^(\d{4}/\d{2}/\d{2}) (\d{2}:\d{2}:\d{2}) \[(\d+)\] total:\s+matches=([\d,]+)\s+hash_hits=([\d,]+)\s+false_alarms=([\d,]+)\s+data=([\d,]+)")
            .map_err(|e| ParseError::Regex(e.to_string()))?;
        let stat2_log_v_re = Regex::new(r"^(\d{4}/\d{2}/\d{2}) (\d{2}:\d{2}:\d{2}) \[(\d+)\] sent\s+([\d,]+)\s+bytes\s+received\s+([\d,]+)\s+bytes\s+([\d,.]+)\s+bytes/sec")
            .map_err(|e| ParseError::Regex(e.to_string()))?;
        let stat3_log_v_re = Regex::new(r"^(\d{4}/\d{2}/\d{2}) (\d{2}:\d{2}:\d{2}) \[(\d+)\] total\s+size\s+is\s+([\d,]+)\s+speedup\s+is\s+([\d,.]+)")
            .map_err(|e| ParseError::Regex(e.to_string()))?;

        let mut results: Vec<Map<String, Value>> = Vec::new();
        let mut summary: Map<String, Value> = Map::new();

        for line in input.lines() {
            if line.trim().is_empty() {
                continue;
            }

            // Log file lines (longer meta format)
            if let Some(caps) = file_line_log_re.captures(line) {
                let meta = caps.get(4).map_or("", |m| m.as_str());
                let name = caps.get(5).map_or("", |m| m.as_str());
                let mut file = parse_file_meta(meta, name);
                file.insert(
                    "date".to_string(),
                    Value::String(caps.get(1).map_or("", |m| m.as_str()).to_string()),
                );
                file.insert(
                    "time".to_string(),
                    Value::String(caps.get(2).map_or("", |m| m.as_str()).to_string()),
                );
                let proc_str = caps.get(3).map_or("", |m| m.as_str());
                file.insert(
                    "process".to_string(),
                    proc_str
                        .parse::<i64>()
                        .map(|n| Value::Number(n.into()))
                        .unwrap_or(Value::Null),
                );
                results.push(file);
                continue;
            }

            if let Some(caps) = file_line_log_mac_re.captures(line) {
                let meta = caps.get(4).map_or("", |m| m.as_str());
                let name = caps.get(5).map_or("", |m| m.as_str());
                let mut file = parse_file_meta(meta, name);
                file.insert(
                    "date".to_string(),
                    Value::String(caps.get(1).map_or("", |m| m.as_str()).to_string()),
                );
                file.insert(
                    "time".to_string(),
                    Value::String(caps.get(2).map_or("", |m| m.as_str()).to_string()),
                );
                let proc_str = caps.get(3).map_or("", |m| m.as_str());
                file.insert(
                    "process".to_string(),
                    proc_str
                        .parse::<i64>()
                        .map(|n| Value::Number(n.into()))
                        .unwrap_or(Value::Null),
                );
                results.push(file);
                continue;
            }

            // Non-log file lines
            if let Some(caps) = file_line_re.captures(line) {
                let meta = caps.get(1).map_or("", |m| m.as_str());
                let name = caps.get(2).map_or("", |m| m.as_str());
                results.push(parse_file_meta(meta, name));
                continue;
            }

            if let Some(caps) = file_line_mac_re.captures(line) {
                let meta = caps.get(1).map_or("", |m| m.as_str());
                let name = caps.get(2).map_or("", |m| m.as_str());
                results.push(parse_file_meta(meta, name));
                continue;
            }

            // Summary lines
            if let Some(caps) = stat1_re.captures(line) {
                summary.insert("type".to_string(), Value::String("summary".to_string()));
                summary.insert(
                    "sent".to_string(),
                    parse_size_to_int(caps.get(1).map_or("", |m| m.as_str()))
                        .map(|n| Value::Number(n.into()))
                        .unwrap_or(Value::Null),
                );
                summary.insert(
                    "received".to_string(),
                    parse_size_to_int(caps.get(2).map_or("", |m| m.as_str()))
                        .map(|n| Value::Number(n.into()))
                        .unwrap_or(Value::Null),
                );
                summary.insert(
                    "bytes_sec".to_string(),
                    parse_size_to_float(caps.get(3).map_or("", |m| m.as_str()))
                        .and_then(|f| serde_json::Number::from_f64(f))
                        .map(Value::Number)
                        .unwrap_or(Value::Null),
                );
                continue;
            }
            if let Some(caps) = stat2_re.captures(line) {
                summary.insert(
                    "total_size".to_string(),
                    parse_size_to_int(caps.get(1).map_or("", |m| m.as_str()))
                        .map(|n| Value::Number(n.into()))
                        .unwrap_or(Value::Null),
                );
                summary.insert(
                    "speedup".to_string(),
                    parse_size_to_float(caps.get(2).map_or("", |m| m.as_str()))
                        .and_then(|f| serde_json::Number::from_f64(f))
                        .map(Value::Number)
                        .unwrap_or(Value::Null),
                );
                continue;
            }
            if let Some(caps) = stat1_simple_re.captures(line) {
                summary.insert("type".to_string(), Value::String("summary".to_string()));
                summary.insert(
                    "sent".to_string(),
                    parse_size_to_int(caps.get(1).map_or("", |m| m.as_str()))
                        .map(|n| Value::Number(n.into()))
                        .unwrap_or(Value::Null),
                );
                summary.insert(
                    "received".to_string(),
                    parse_size_to_int(caps.get(2).map_or("", |m| m.as_str()))
                        .map(|n| Value::Number(n.into()))
                        .unwrap_or(Value::Null),
                );
                summary.insert(
                    "bytes_sec".to_string(),
                    parse_size_to_float(caps.get(3).map_or("", |m| m.as_str()))
                        .and_then(|f| serde_json::Number::from_f64(f))
                        .map(Value::Number)
                        .unwrap_or(Value::Null),
                );
                continue;
            }
            if let Some(caps) = stat2_simple_re.captures(line) {
                summary.insert(
                    "total_size".to_string(),
                    parse_size_to_int(caps.get(1).map_or("", |m| m.as_str()))
                        .map(|n| Value::Number(n.into()))
                        .unwrap_or(Value::Null),
                );
                summary.insert(
                    "speedup".to_string(),
                    parse_size_to_float(caps.get(2).map_or("", |m| m.as_str()))
                        .and_then(|f| serde_json::Number::from_f64(f))
                        .map(Value::Number)
                        .unwrap_or(Value::Null),
                );
                continue;
            }

            // Log format summaries
            if let Some(caps) = stat_log_re.captures(line) {
                summary.insert("type".to_string(), Value::String("summary".to_string()));
                summary.insert(
                    "date".to_string(),
                    Value::String(caps.get(1).map_or("", |m| m.as_str()).to_string()),
                );
                summary.insert(
                    "time".to_string(),
                    Value::String(caps.get(2).map_or("", |m| m.as_str()).to_string()),
                );
                summary.insert(
                    "process".to_string(),
                    caps.get(3)
                        .and_then(|m| m.as_str().parse::<i64>().ok())
                        .map(|n| Value::Number(n.into()))
                        .unwrap_or(Value::Null),
                );
                summary.insert(
                    "sent".to_string(),
                    parse_size_to_int(caps.get(4).map_or("", |m| m.as_str()))
                        .map(|n| Value::Number(n.into()))
                        .unwrap_or(Value::Null),
                );
                summary.insert(
                    "received".to_string(),
                    parse_size_to_int(caps.get(5).map_or("", |m| m.as_str()))
                        .map(|n| Value::Number(n.into()))
                        .unwrap_or(Value::Null),
                );
                summary.insert(
                    "total_size".to_string(),
                    parse_size_to_int(caps.get(6).map_or("", |m| m.as_str()))
                        .map(|n| Value::Number(n.into()))
                        .unwrap_or(Value::Null),
                );
                continue;
            }

            if let Some(caps) = stat1_log_v_re.captures(line) {
                summary.insert("type".to_string(), Value::String("summary".to_string()));
                summary.insert(
                    "date".to_string(),
                    Value::String(caps.get(1).map_or("", |m| m.as_str()).to_string()),
                );
                summary.insert(
                    "time".to_string(),
                    Value::String(caps.get(2).map_or("", |m| m.as_str()).to_string()),
                );
                summary.insert(
                    "process".to_string(),
                    caps.get(3)
                        .and_then(|m| m.as_str().parse::<i64>().ok())
                        .map(|n| Value::Number(n.into()))
                        .unwrap_or(Value::Null),
                );
                summary.insert(
                    "matches".to_string(),
                    parse_size_to_int(caps.get(4).map_or("", |m| m.as_str()))
                        .map(|n| Value::Number(n.into()))
                        .unwrap_or(Value::Null),
                );
                summary.insert(
                    "hash_hits".to_string(),
                    parse_size_to_int(caps.get(5).map_or("", |m| m.as_str()))
                        .map(|n| Value::Number(n.into()))
                        .unwrap_or(Value::Null),
                );
                summary.insert(
                    "false_alarms".to_string(),
                    parse_size_to_int(caps.get(6).map_or("", |m| m.as_str()))
                        .map(|n| Value::Number(n.into()))
                        .unwrap_or(Value::Null),
                );
                summary.insert(
                    "data".to_string(),
                    parse_size_to_int(caps.get(7).map_or("", |m| m.as_str()))
                        .map(|n| Value::Number(n.into()))
                        .unwrap_or(Value::Null),
                );
                continue;
            }
            if let Some(caps) = stat2_log_v_re.captures(line) {
                summary.insert(
                    "sent".to_string(),
                    parse_size_to_int(caps.get(4).map_or("", |m| m.as_str()))
                        .map(|n| Value::Number(n.into()))
                        .unwrap_or(Value::Null),
                );
                summary.insert(
                    "received".to_string(),
                    parse_size_to_int(caps.get(5).map_or("", |m| m.as_str()))
                        .map(|n| Value::Number(n.into()))
                        .unwrap_or(Value::Null),
                );
                summary.insert(
                    "bytes_sec".to_string(),
                    parse_size_to_float(caps.get(6).map_or("", |m| m.as_str()))
                        .and_then(|f| serde_json::Number::from_f64(f))
                        .map(Value::Number)
                        .unwrap_or(Value::Null),
                );
                continue;
            }
            if let Some(caps) = stat3_log_v_re.captures(line) {
                summary.insert(
                    "total_size".to_string(),
                    parse_size_to_int(caps.get(4).map_or("", |m| m.as_str()))
                        .map(|n| Value::Number(n.into()))
                        .unwrap_or(Value::Null),
                );
                summary.insert(
                    "speedup".to_string(),
                    parse_size_to_float(caps.get(5).map_or("", |m| m.as_str()))
                        .and_then(|f| serde_json::Number::from_f64(f))
                        .map(Value::Number)
                        .unwrap_or(Value::Null),
                );
                continue;
            }
        }

        // Append summary at end if present
        if !summary.is_empty() {
            results.push(summary);
        }

        Ok(ParseOutput::Array(results))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jc_rs_core::traits::Parser;

    #[test]
    fn test_rsync_s_i_golden() {
        let input = include_str!("../../../../tests/fixtures/centos-7.7/rsync-i.out");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/centos-7.7/rsync-i-streaming.json"
        ))
        .unwrap();
        let result = RsyncStreamParser.parse(input, false).unwrap();
        let actual = serde_json::to_value(result).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_rsync_s_empty() {
        let result = RsyncStreamParser.parse("", false).unwrap();
        assert!(matches!(result, ParseOutput::Array(v) if v.is_empty()));
    }

    #[test]
    fn test_rsync_s_registered() {
        assert!(jc_rs_core::registry::find_parser("rsync_s").is_some());
    }
}
