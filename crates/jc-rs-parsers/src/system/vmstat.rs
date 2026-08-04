//! Parser for `vmstat` command output.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use jc_rs_utils::{convert_to_int, parse_timestamp};
use regex::Regex;
use serde_json::{Map, Value};
use std::sync::OnceLock;

fn procs_header_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^-*procs-* ").unwrap())
}

fn disk_header_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^-*disk-* ").unwrap())
}

pub struct VmstatParser;

static INFO: ParserInfo = ParserInfo {
    name: "vmstat",
    argument: "--vmstat",
    version: "1.4.0",
    description: "Converts `vmstat` command output to JSON",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[Platform::Linux],
    tags: &[Tag::Command],
    magic_commands: &["vmstat"],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static VMSTAT_PARSER: VmstatParser = VmstatParser;

inventory::submit! {
    ParserEntry::new(&VMSTAT_PARSER)
}

impl Parser for VmstatParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        let rows = parse_vmstat(input);
        Ok(ParseOutput::Array(rows))
    }
}

fn int_val(s: Option<&&str>) -> Value {
    match s {
        Some(v) => convert_to_int(v)
            .map(|n| Value::Number(n.into()))
            .unwrap_or(Value::Null),
        None => Value::Null,
    }
}

fn opt_str(s: Option<&&str>) -> Value {
    match s {
        Some(v) => Value::String(v.to_string()),
        None => Value::Null,
    }
}

pub fn parse_vmstat(input: &str) -> Vec<Map<String, Value>> {
    let mut raw_output: Vec<Map<String, Value>> = Vec::new();
    let mut procs: Option<bool> = None; // Some(true) = procs mode detected
    let mut disk: Option<bool> = None; // Some(true) = disk mode detected
    let mut buff_cache: Option<bool> = None; // true = buff/cache mode, false = inact/active mode
    let mut tstamp = false;
    let mut tz: Option<String> = None;

    for line in input.lines() {
        if line.trim().is_empty() {
            continue;
        }

        // Detect output type
        if procs.is_none() && disk.is_none() && procs_header_re().is_match(line) {
            procs = Some(true);
            tstamp = line.contains("-timestamp-");
            continue;
        }

        if procs.is_none() && disk.is_none() && disk_header_re().is_match(line) {
            disk = Some(true);
            tstamp = line.contains("-timestamp-");
            continue;
        }

        // Skip header rows
        if (procs.is_some() || disk.is_some())
            && (procs_header_re().is_match(line) || disk_header_re().is_match(line))
        {
            continue;
        }

        // Detect buff/cache vs inact/active column header
        if line.contains("swpd")
            && line.contains("free")
            && line.contains("buff")
            && line.contains("cache")
        {
            buff_cache = Some(true);
            if tstamp {
                tz = line.split_whitespace().last().map(|s| s.to_string());
            }
            continue;
        }

        if line.contains("swpd")
            && line.contains("free")
            && line.contains("inact")
            && line.contains("active")
        {
            buff_cache = Some(false);
            if tstamp {
                tz = line.split_whitespace().last().map(|s| s.to_string());
            }
            continue;
        }

        // Disk header row
        if line.contains("total") && line.contains("merged") && line.contains("sectors") {
            if tstamp {
                tz = line.split_whitespace().last().map(|s| s.to_string());
            }
            continue;
        }

        // Data line parsing
        if procs.is_some() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 17 {
                continue;
            }
            let ts_val = if tstamp && parts.len() > 17 {
                // timestamp may have a space (e.g., "2021-09-16 20:33:13")
                Some(parts[17..].join(" "))
            } else {
                None
            };
            let bc = buff_cache.unwrap_or(true);
            let p = &parts;

            let mut record = Map::new();
            record.insert("runnable_procs".to_string(), int_val(p.get(0)));
            record.insert(
                "uninterruptible_sleeping_procs".to_string(),
                int_val(p.get(1)),
            );
            record.insert("virtual_mem_used".to_string(), int_val(p.get(2)));
            record.insert("free_mem".to_string(), int_val(p.get(3)));
            record.insert(
                "buffer_mem".to_string(),
                if bc { int_val(p.get(4)) } else { Value::Null },
            );
            record.insert(
                "cache_mem".to_string(),
                if bc { int_val(p.get(5)) } else { Value::Null },
            );
            record.insert(
                "inactive_mem".to_string(),
                if !bc { int_val(p.get(4)) } else { Value::Null },
            );
            record.insert(
                "active_mem".to_string(),
                if !bc { int_val(p.get(5)) } else { Value::Null },
            );
            record.insert("swap_in".to_string(), int_val(p.get(6)));
            record.insert("swap_out".to_string(), int_val(p.get(7)));
            record.insert("blocks_in".to_string(), int_val(p.get(8)));
            record.insert("blocks_out".to_string(), int_val(p.get(9)));
            record.insert("interrupts".to_string(), int_val(p.get(10)));
            record.insert("context_switches".to_string(), int_val(p.get(11)));
            record.insert("user_time".to_string(), int_val(p.get(12)));
            record.insert("system_time".to_string(), int_val(p.get(13)));
            record.insert("idle_time".to_string(), int_val(p.get(14)));
            record.insert("io_wait_time".to_string(), int_val(p.get(15)));
            record.insert("stolen_time".to_string(), int_val(p.get(16)));
            record.insert(
                "timestamp".to_string(),
                ts_val
                    .as_deref()
                    .map(|s| Value::String(s.to_string()))
                    .unwrap_or(Value::Null),
            );
            record.insert(
                "timezone".to_string(),
                tz.as_deref()
                    .map(|s| Value::String(s.to_string()))
                    .unwrap_or(Value::Null),
            );
            if let Some(ref ts) = ts_val {
                let ts_with_tz = match &tz {
                    Some(t) => format!("{} {}", ts, t),
                    None => ts.clone(),
                };
                let parsed = parse_timestamp(&ts_with_tz, None);
                record.insert(
                    "epoch".to_string(),
                    parsed
                        .naive_epoch
                        .map(|e| Value::Number(e.into()))
                        .unwrap_or(Value::Null),
                );
                record.insert(
                    "epoch_utc".to_string(),
                    parsed
                        .utc_epoch
                        .map(|e| Value::Number(e.into()))
                        .unwrap_or(Value::Null),
                );
            }
            raw_output.push(record);
        }

        if disk.is_some() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 11 {
                continue;
            }
            let ts_val: Option<String> = if tstamp && parts.len() > 11 {
                Some(parts[11..].join(" "))
            } else {
                None
            };
            let p = &parts;

            let mut record = Map::new();
            record.insert("disk".to_string(), opt_str(p.get(0)));
            record.insert("total_reads".to_string(), int_val(p.get(1)));
            record.insert("merged_reads".to_string(), int_val(p.get(2)));
            record.insert("sectors_read".to_string(), int_val(p.get(3)));
            record.insert("reading_ms".to_string(), int_val(p.get(4)));
            record.insert("total_writes".to_string(), int_val(p.get(5)));
            record.insert("merged_writes".to_string(), int_val(p.get(6)));
            record.insert("sectors_written".to_string(), int_val(p.get(7)));
            record.insert("writing_ms".to_string(), int_val(p.get(8)));
            record.insert("current_io".to_string(), int_val(p.get(9)));
            record.insert("io_seconds".to_string(), int_val(p.get(10)));
            record.insert(
                "timestamp".to_string(),
                ts_val
                    .as_deref()
                    .map(|s| Value::String(s.to_string()))
                    .unwrap_or(Value::Null),
            );
            record.insert(
                "timezone".to_string(),
                tz.as_deref()
                    .map(|s| Value::String(s.to_string()))
                    .unwrap_or(Value::Null),
            );
            if let Some(ref ts) = ts_val {
                let ts_with_tz = match &tz {
                    Some(t) => format!("{} {}", ts, t),
                    None => ts.clone(),
                };
                let parsed = parse_timestamp(&ts_with_tz, None);
                record.insert(
                    "epoch".to_string(),
                    parsed
                        .naive_epoch
                        .map(|e| Value::Number(e.into()))
                        .unwrap_or(Value::Null),
                );
                record.insert(
                    "epoch_utc".to_string(),
                    parsed
                        .utc_epoch
                        .map(|e| Value::Number(e.into()))
                        .unwrap_or(Value::Null),
                );
            }
            raw_output.push(record);
        }
    }

    raw_output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vmstat_centos() {
        let input = include_str!("../../../../tests/fixtures/centos-7.7/vmstat.out");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/centos-7.7/vmstat.json"
        ))
        .unwrap();
        let parser = VmstatParser;
        let result = parser.parse(input, false).unwrap();
        let result_value: serde_json::Value = serde_json::to_value(result).unwrap();
        assert_eq!(result_value, expected);
    }

    #[test]
    fn test_vmstat_a_centos() {
        let input = include_str!("../../../../tests/fixtures/centos-7.7/vmstat-a.out");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/centos-7.7/vmstat-a.json"
        ))
        .unwrap();
        let parser = VmstatParser;
        let result = parser.parse(input, false).unwrap();
        let result_value: serde_json::Value = serde_json::to_value(result).unwrap();
        assert_eq!(result_value, expected);
    }

    #[test]
    fn test_vmstat_d_centos() {
        let input = include_str!("../../../../tests/fixtures/centos-7.7/vmstat-d.out");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/centos-7.7/vmstat-d.json"
        ))
        .unwrap();
        let parser = VmstatParser;
        let result = parser.parse(input, false).unwrap();
        let result_value: serde_json::Value = serde_json::to_value(result).unwrap();
        assert_eq!(result_value, expected);
    }

    #[test]
    fn test_vmstat_w_centos() {
        let input = include_str!("../../../../tests/fixtures/centos-7.7/vmstat-w.out");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/centos-7.7/vmstat-w.json"
        ))
        .unwrap();
        let parser = VmstatParser;
        let result = parser.parse(input, false).unwrap();
        let result_value: serde_json::Value = serde_json::to_value(result).unwrap();
        assert_eq!(result_value, expected);
    }
}
