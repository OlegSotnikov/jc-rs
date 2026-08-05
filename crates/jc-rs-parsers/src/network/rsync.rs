//! Parser for `rsync` command output (non-streaming, batch mode).

use chrono::NaiveDateTime;
use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use regex::Regex;
use serde_json::{Map, Value};

pub struct RsyncParser;

static INFO: ParserInfo = ParserInfo {
    name: "rsync",
    argument: "--rsync",
    version: "1.3.0",
    description: "Converts `rsync` command output to JSON",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[Platform::Linux, Platform::Darwin, Platform::FreeBSD],
    tags: &[Tag::Command],
    magic_commands: &["rsync"],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static RSYNC_PARSER: RsyncParser = RsyncParser;
inventory::submit! { ParserEntry::new(&RSYNC_PARSER) }

/// rsync writes sizes as `8.99K` / `6.88T`, and jc reads those with decimal
/// multipliers: 8.99K is 8990 bytes, not 9205.
fn parse_size_to_int(s: &str) -> Option<i64> {
    jc_rs_utils::convert_size_to_int(s, false)
}

fn parse_size_to_float(s: &str) -> Option<f64> {
    let cleaned = s.replace(',', "");
    let cleaned = cleaned.trim();
    if cleaned.is_empty() {
        return None;
    }
    if cleaned.ends_with(['K', 'M', 'G', 'T']) {
        return jc_rs_utils::convert_size_to_int(cleaned, false).map(|n| n as f64);
    }
    cleaned.parse::<f64>().ok()
}

fn compute_epoch(date: &str, time: &str) -> Option<i64> {
    let date = date.replace('/', "-");
    let dt_str = format!("{} {}", date, time);
    let ndt = NaiveDateTime::parse_from_str(&dt_str, "%Y-%m-%d %H:%M:%S").ok()?;
    Some(
        ndt.and_local_timezone(chrono::Local)
            .earliest()?
            .timestamp(),
    )
}

fn parse_file_line(meta: &str, name: &str) -> Map<String, Value> {
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

    fn flag_bool(c: char, true_char: char) -> Value {
        match c {
            _ if c == true_char => Value::Bool(true),
            '.' => Value::Bool(false),
            '+' | ' ' | '?' => Value::Null,
            _ => Value::Null,
        }
    }

    let update_type = update_type_map
        .iter()
        .find(|(k, _)| meta.starts_with(k))
        .map(|(_, v)| {
            v.map(|s| Value::String(s.to_string()))
                .unwrap_or(Value::Null)
        })
        .unwrap_or(Value::Null);

    let file_type = if meta.len() > 1 {
        let ft_char = meta.chars().nth(1).unwrap_or('+');
        file_type_map
            .iter()
            .find(|(k, _)| k.starts_with(ft_char))
            .map(|(_, v)| {
                v.map(|s| Value::String(s.to_string()))
                    .unwrap_or(Value::Null)
            })
            .unwrap_or(Value::Null)
    } else {
        Value::Null
    };

    let chars: Vec<char> = meta.chars().collect();
    let get = |i: usize| chars.get(i).copied().unwrap_or('+');

    let is_mac_format = meta.len() == 9; // Mac uses 9-char metadata

    let mut obj = Map::new();
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
        // Linux format: positions 8,9,10 are u, acl, extended attrs
        obj.insert("acl_different".to_string(), flag_bool(get(9), 'a'));
        obj.insert(
            "extended_attribute_different".to_string(),
            flag_bool(get(10), 'x'),
        );
    }

    obj
}

impl Parser for RsyncParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Array(vec![]));
        }

        // Regexes for file lines and summary
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

        // Group by process ID for log format
        let mut groups: Vec<(String, Vec<Map<String, Value>>, Map<String, Value>)> = Vec::new(); // (process_id, files, summary)
        let mut current_process = String::new();
        let mut current_files: Vec<Map<String, Value>> = Vec::new();
        let mut current_summary: Map<String, Value> = Map::new();
        let mut summary_partial: Map<String, Value> = Map::new();
        let mut is_log_format = false;

        for line in input.lines() {
            if line.trim().is_empty() {
                continue;
            }

            // Try log format first
            if let Some(caps) = file_line_log_re.captures(line) {
                is_log_format = true;
                let process = caps.get(3).map_or("", |m| m.as_str()).to_string();
                if !current_process.is_empty() && process != current_process {
                    // New process - save current group
                    groups.push((
                        current_process.clone(),
                        current_files.clone(),
                        current_summary.clone(),
                    ));
                    current_files.clear();
                    current_summary = Map::new();
                    summary_partial = Map::new();
                }
                current_process = process.clone();
                let date_str = caps.get(1).map_or("", |m| m.as_str());
                let time_str = caps.get(2).map_or("", |m| m.as_str());
                let meta = caps.get(4).map_or("", |m| m.as_str());
                let name = caps.get(5).map_or("", |m| m.as_str());
                let mut file = parse_file_line(meta, name);
                file.insert("date".to_string(), Value::String(date_str.to_string()));
                file.insert("time".to_string(), Value::String(time_str.to_string()));
                file.insert(
                    "process".to_string(),
                    process
                        .parse::<i64>()
                        .map(|n| Value::Number(n.into()))
                        .unwrap_or(Value::Null),
                );
                if let Some(epoch) = compute_epoch(date_str, time_str) {
                    file.insert("epoch".to_string(), Value::Number(epoch.into()));
                } else {
                    file.insert("epoch".to_string(), Value::Null);
                }
                current_files.push(file);
                continue;
            }

            if let Some(caps) = file_line_log_mac_re.captures(line) {
                is_log_format = true;
                let process = caps.get(3).map_or("", |m| m.as_str()).to_string();
                if !current_process.is_empty() && process != current_process {
                    groups.push((
                        current_process.clone(),
                        current_files.clone(),
                        current_summary.clone(),
                    ));
                    current_files.clear();
                    current_summary = Map::new();
                    summary_partial = Map::new();
                }
                current_process = process.clone();
                let date_str = caps.get(1).map_or("", |m| m.as_str());
                let time_str = caps.get(2).map_or("", |m| m.as_str());
                let meta = caps.get(4).map_or("", |m| m.as_str());
                let name = caps.get(5).map_or("", |m| m.as_str());
                let mut file = parse_file_line(meta, name);
                file.insert("date".to_string(), Value::String(date_str.to_string()));
                file.insert("time".to_string(), Value::String(time_str.to_string()));
                file.insert(
                    "process".to_string(),
                    process
                        .parse::<i64>()
                        .map(|n| Value::Number(n.into()))
                        .unwrap_or(Value::Null),
                );
                if let Some(epoch) = compute_epoch(date_str, time_str) {
                    file.insert("epoch".to_string(), Value::Number(epoch.into()));
                } else {
                    file.insert("epoch".to_string(), Value::Null);
                }
                current_files.push(file);
                continue;
            }

            // Non-log file lines
            if let Some(caps) = file_line_re.captures(line) {
                let meta = caps.get(1).map_or("", |m| m.as_str());
                let name = caps.get(2).map_or("", |m| m.as_str());
                current_files.push(parse_file_line(meta, name));
                continue;
            }

            if let Some(caps) = file_line_mac_re.captures(line) {
                let meta = caps.get(1).map_or("", |m| m.as_str());
                let name = caps.get(2).map_or("", |m| m.as_str());
                current_files.push(parse_file_line(meta, name));
                continue;
            }

            // Summary lines
            if let Some(caps) = stat1_re.captures(line) {
                summary_partial.insert(
                    "sent".to_string(),
                    parse_size_to_int(caps.get(1).map_or("", |m| m.as_str()))
                        .map(|n| Value::Number(n.into()))
                        .unwrap_or(Value::Null),
                );
                summary_partial.insert(
                    "received".to_string(),
                    parse_size_to_int(caps.get(2).map_or("", |m| m.as_str()))
                        .map(|n| Value::Number(n.into()))
                        .unwrap_or(Value::Null),
                );
                summary_partial.insert(
                    "bytes_sec".to_string(),
                    parse_size_to_float(caps.get(3).map_or("", |m| m.as_str()))
                        .and_then(serde_json::Number::from_f64)
                        .map(Value::Number)
                        .unwrap_or(Value::Null),
                );
                continue;
            }
            if let Some(caps) = stat2_re.captures(line) {
                summary_partial.insert(
                    "total_size".to_string(),
                    parse_size_to_int(caps.get(1).map_or("", |m| m.as_str()))
                        .map(|n| Value::Number(n.into()))
                        .unwrap_or(Value::Null),
                );
                summary_partial.insert(
                    "speedup".to_string(),
                    parse_size_to_float(caps.get(2).map_or("", |m| m.as_str()))
                        .and_then(serde_json::Number::from_f64)
                        .map(Value::Number)
                        .unwrap_or(Value::Null),
                );
                // Finalize summary
                current_summary = summary_partial.clone();
                continue;
            }
            if let Some(caps) = stat1_simple_re.captures(line) {
                summary_partial.insert(
                    "sent".to_string(),
                    parse_size_to_int(caps.get(1).map_or("", |m| m.as_str()))
                        .map(|n| Value::Number(n.into()))
                        .unwrap_or(Value::Null),
                );
                summary_partial.insert(
                    "received".to_string(),
                    parse_size_to_int(caps.get(2).map_or("", |m| m.as_str()))
                        .map(|n| Value::Number(n.into()))
                        .unwrap_or(Value::Null),
                );
                summary_partial.insert(
                    "bytes_sec".to_string(),
                    parse_size_to_float(caps.get(3).map_or("", |m| m.as_str()))
                        .and_then(serde_json::Number::from_f64)
                        .map(Value::Number)
                        .unwrap_or(Value::Null),
                );
                continue;
            }
            if let Some(caps) = stat2_simple_re.captures(line) {
                summary_partial.insert(
                    "total_size".to_string(),
                    parse_size_to_int(caps.get(1).map_or("", |m| m.as_str()))
                        .map(|n| Value::Number(n.into()))
                        .unwrap_or(Value::Null),
                );
                summary_partial.insert(
                    "speedup".to_string(),
                    parse_size_to_float(caps.get(2).map_or("", |m| m.as_str()))
                        .and_then(serde_json::Number::from_f64)
                        .map(Value::Number)
                        .unwrap_or(Value::Null),
                );
                current_summary = summary_partial.clone();
                continue;
            }

            // Log format summary lines
            if let Some(caps) = stat_log_re.captures(line) {
                summary_partial.insert(
                    "date".to_string(),
                    Value::String(caps.get(1).map_or("", |m| m.as_str()).to_string()),
                );
                summary_partial.insert(
                    "time".to_string(),
                    Value::String(caps.get(2).map_or("", |m| m.as_str()).to_string()),
                );
                summary_partial.insert(
                    "process".to_string(),
                    caps.get(3)
                        .and_then(|m| m.as_str().parse::<i64>().ok())
                        .map(|n| Value::Number(n.into()))
                        .unwrap_or(Value::Null),
                );
                summary_partial.insert(
                    "sent".to_string(),
                    parse_size_to_int(caps.get(4).map_or("", |m| m.as_str()))
                        .map(|n| Value::Number(n.into()))
                        .unwrap_or(Value::Null),
                );
                summary_partial.insert(
                    "received".to_string(),
                    parse_size_to_int(caps.get(5).map_or("", |m| m.as_str()))
                        .map(|n| Value::Number(n.into()))
                        .unwrap_or(Value::Null),
                );
                summary_partial.insert(
                    "total_size".to_string(),
                    parse_size_to_int(caps.get(6).map_or("", |m| m.as_str()))
                        .map(|n| Value::Number(n.into()))
                        .unwrap_or(Value::Null),
                );
                current_summary = summary_partial.clone();
                continue;
            }

            if let Some(caps) = stat1_log_v_re.captures(line) {
                summary_partial.clear();
                summary_partial.insert(
                    "date".to_string(),
                    Value::String(caps.get(1).map_or("", |m| m.as_str()).to_string()),
                );
                summary_partial.insert(
                    "time".to_string(),
                    Value::String(caps.get(2).map_or("", |m| m.as_str()).to_string()),
                );
                summary_partial.insert(
                    "process".to_string(),
                    caps.get(3)
                        .and_then(|m| m.as_str().parse::<i64>().ok())
                        .map(|n| Value::Number(n.into()))
                        .unwrap_or(Value::Null),
                );
                summary_partial.insert(
                    "matches".to_string(),
                    parse_size_to_int(caps.get(4).map_or("", |m| m.as_str()))
                        .map(|n| Value::Number(n.into()))
                        .unwrap_or(Value::Null),
                );
                summary_partial.insert(
                    "hash_hits".to_string(),
                    parse_size_to_int(caps.get(5).map_or("", |m| m.as_str()))
                        .map(|n| Value::Number(n.into()))
                        .unwrap_or(Value::Null),
                );
                summary_partial.insert(
                    "false_alarms".to_string(),
                    parse_size_to_int(caps.get(6).map_or("", |m| m.as_str()))
                        .map(|n| Value::Number(n.into()))
                        .unwrap_or(Value::Null),
                );
                summary_partial.insert(
                    "data".to_string(),
                    parse_size_to_int(caps.get(7).map_or("", |m| m.as_str()))
                        .map(|n| Value::Number(n.into()))
                        .unwrap_or(Value::Null),
                );
                continue;
            }
            if let Some(caps) = stat2_log_v_re.captures(line) {
                summary_partial.insert(
                    "sent".to_string(),
                    parse_size_to_int(caps.get(4).map_or("", |m| m.as_str()))
                        .map(|n| Value::Number(n.into()))
                        .unwrap_or(Value::Null),
                );
                summary_partial.insert(
                    "received".to_string(),
                    parse_size_to_int(caps.get(5).map_or("", |m| m.as_str()))
                        .map(|n| Value::Number(n.into()))
                        .unwrap_or(Value::Null),
                );
                summary_partial.insert(
                    "bytes_sec".to_string(),
                    parse_size_to_float(caps.get(6).map_or("", |m| m.as_str()))
                        .and_then(serde_json::Number::from_f64)
                        .map(Value::Number)
                        .unwrap_or(Value::Null),
                );
                continue;
            }
            if crate::network::rsync_s::apply_stat_ex(line, &mut summary_partial) {
                current_summary = summary_partial.clone();
                continue;
            }
            if let Some(caps) = stat3_log_v_re.captures(line) {
                summary_partial.insert(
                    "total_size".to_string(),
                    parse_size_to_int(caps.get(4).map_or("", |m| m.as_str()))
                        .map(|n| Value::Number(n.into()))
                        .unwrap_or(Value::Null),
                );
                summary_partial.insert(
                    "speedup".to_string(),
                    parse_size_to_float(caps.get(5).map_or("", |m| m.as_str()))
                        .and_then(serde_json::Number::from_f64)
                        .map(Value::Number)
                        .unwrap_or(Value::Null),
                );
                current_summary = summary_partial.clone();
                continue;
            }
        }

        // Build output: array of {summary, files} objects
        // If log format with multiple processes, we have multiple groups
        if !is_log_format || groups.is_empty() {
            // Single group
            let mut group = Map::new();
            group.insert("summary".to_string(), Value::Object(current_summary));
            group.insert(
                "files".to_string(),
                Value::Array(current_files.into_iter().map(Value::Object).collect()),
            );
            Ok(ParseOutput::Array(vec![group]))
        } else {
            // Multiple groups
            groups.push((current_process, current_files, current_summary));
            let result: Vec<Map<String, Value>> = groups
                .into_iter()
                .map(|(_, files, summary)| {
                    let mut group = Map::new();
                    group.insert("summary".to_string(), Value::Object(summary));
                    group.insert(
                        "files".to_string(),
                        Value::Array(files.into_iter().map(Value::Object).collect()),
                    );
                    group
                })
                .collect();
            Ok(ParseOutput::Array(result))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jc_rs_core::traits::Parser;

    #[test]
    fn test_rsync_i_golden() {
        let input = include_str!("../../../../tests/fixtures/centos-7.7/rsync-i.out");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/centos-7.7/rsync-i.json"
        ))
        .unwrap();
        let result = RsyncParser.parse(input, false).unwrap();
        let actual = serde_json::to_value(result).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_rsync_empty() {
        let result = RsyncParser.parse("", false).unwrap();
        assert!(matches!(result, ParseOutput::Array(v) if v.is_empty()));
    }

    #[test]
    fn test_rsync_registered() {
        assert!(jc_rs_core::registry::find_parser("rsync").is_some());
    }
}
