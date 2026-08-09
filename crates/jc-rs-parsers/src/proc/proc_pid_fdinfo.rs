//! Parser for `/proc/<pid>/fdinfo/<fd>`

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};

pub struct ProcPidFdinfoParser;

static INFO: ParserInfo = ParserInfo {
    name: "proc_pid_fdinfo",
    argument: "--proc-pid-fdinfo",
    version: "1.0.0",
    description: "Converts `/proc/<pid>/fdinfo/<fd>` file to JSON",
    author: "jc-rs contributors",
    author_email: "os@g1sw.com",
    compatible: &[Platform::Linux],
    tags: &[Tag::File, Tag::Slurpable],
    magic_commands: &["/proc/<pid>/fdinfo/<fd>"],
    streaming: false,
    hidden: true,
    deprecated: false,
};

static INSTANCE: ProcPidFdinfoParser = ProcPidFdinfoParser;
inventory::submit! { ParserEntry::new(&INSTANCE) }

/// Fields at the root level that should be converted to integers.
const ROOT_INT_FIELDS: &[&str] = &[
    "pos",
    "flags",
    "mnt_id",
    "ino",
    "clockid",
    "ticks",
    "settime flags",
    "size",
    "count",
];

/// Fields inside the "epoll" nested object that should be integers.
const EPOLL_INT_FIELDS: &[&str] = &["tfd", "pos"];

/// Fields inside the "inotify" nested object that should be integers.
const INOTIFY_INT_FIELDS: &[&str] = &["wd"];

/// Parse a "(sec,nsec)" string into a Vec<i64>.
fn parse_time_tuple(s: &str) -> Vec<Value> {
    // Format: "(0, 49406829)" or "(1, 0)"
    let cleaned = s.replace(['(', ')', ','], "");
    cleaned
        .split_whitespace()
        .filter_map(|x| x.parse::<i64>().ok())
        .map(|n| Value::Number(n.into()))
        .collect()
}

impl Parser for ProcPidFdinfoParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Object(Map::new()));
        }

        let mut map: Map<String, Value> = Map::new();

        for line in input.lines() {
            let line_trimmed = line.trim_end();
            if line_trimmed.is_empty() {
                continue;
            }

            // epoll: line starts with "tfd:"
            if line_trimmed.starts_with("tfd:") {
                // Parse all "key:value" tokens from the line using regex-like split
                let mut epoll_map: Map<String, Value> = Map::new();
                // tokenize by splitting on whitespace and looking for key:val pairs
                // Format: "tfd:    5 events:       1d data: ffffffffffffffff pos:0 ino:61af sdev:7"
                // Use a simple approach: split on whitespace, look for tokens containing ':'
                // that don't split into empty key/val
                let tokens: Vec<&str> = line_trimmed.split_whitespace().collect();
                let mut i = 0;
                while i < tokens.len() {
                    let tok = tokens[i];
                    if let Some(colon_pos) = tok.find(':') {
                        let k = &tok[..colon_pos];
                        let v = &tok[colon_pos + 1..];
                        if !k.is_empty() {
                            if !v.is_empty() {
                                // value is in same token
                                epoll_map.insert(k.to_string(), Value::String(v.to_string()));
                            } else {
                                // value is in next token
                                i += 1;
                                if let Some(&next) = tokens.get(i) {
                                    epoll_map
                                        .insert(k.to_string(), Value::String(next.to_string()));
                                }
                            }
                        }
                    }
                    i += 1;
                }
                // Convert integer fields in epoll
                for field in EPOLL_INT_FIELDS {
                    if let Some(v) = epoll_map.get(*field)
                        && let Value::String(s) = v
                        && let Ok(n) = s.parse::<i64>()
                    {
                        epoll_map.insert(field.to_string(), Value::Number(n.into()));
                    }
                }
                map.insert("epoll".to_string(), Value::Object(epoll_map));
                continue;
            }

            // inotify: line starts with "inotify"
            if line_trimmed.starts_with("inotify") {
                // Format: "inotify\twd:3 ino:9e7e sdev:800013 mask:800afce ignored_mask:0 ..."
                let rest = &line_trimmed[7..]; // skip "inotify"
                let rest = rest.trim_start();
                let mut inotify_map: Map<String, Value> = Map::new();
                for item in rest.split_whitespace() {
                    if let Some((k, v)) = item.split_once(':') {
                        inotify_map.insert(k.to_string(), Value::String(v.to_string()));
                    }
                }
                // Convert integer fields in inotify
                for field in INOTIFY_INT_FIELDS {
                    if let Some(v) = inotify_map.get(*field)
                        && let Value::String(s) = v
                        && let Ok(n) = s.parse::<i64>()
                    {
                        inotify_map.insert(field.to_string(), Value::Number(n.into()));
                    }
                }
                map.insert("inotify".to_string(), Value::Object(inotify_map));
                continue;
            }

            // fanotify: line starts with "fanotify"
            if line_trimmed.starts_with("fanotify") {
                // May have multiple "fanotify" lines: merge into one object
                let rest = &line_trimmed[8..]; // skip "fanotify"
                let rest = rest.trim_start();
                let fanotify_map = map
                    .entry("fanotify".to_string())
                    .or_insert_with(|| Value::Object(Map::new()));
                if let Value::Object(fm) = fanotify_map {
                    for item in rest.split_whitespace() {
                        if let Some((k, v)) = item.split_once(':') {
                            fm.insert(k.to_string(), Value::String(v.to_string()));
                        }
                    }
                }
                continue;
            }

            // timerfd: it_value and it_interval lines
            // Format: "it_value: (0, 49406829)"
            let first_token = line_trimmed.split_whitespace().next().unwrap_or("");
            if first_token == "it_value:" || first_token == "it_interval:" {
                // key is everything before first ':', value is the rest
                if let Some(colon) = line_trimmed.find(':') {
                    let key = line_trimmed[..colon].trim().to_string();
                    let val_str = line_trimmed[colon + 1..].trim();
                    let arr = parse_time_tuple(val_str);
                    map.insert(key, Value::Array(arr));
                }
                continue;
            }

            // Generic key: value line
            if let Some(colon) = line_trimmed.find(':') {
                let key = line_trimmed[..colon].trim().to_string();
                let val = line_trimmed[colon + 1..].trim().to_string();
                map.insert(key, Value::String(val));
            }
        }

        // Convert root-level integer fields
        for field in ROOT_INT_FIELDS {
            if let Some(v) = map.get(*field)
                && let Value::String(s) = v
                && let Ok(n) = s.parse::<i64>()
            {
                map.insert(field.to_string(), Value::Number(n.into()));
            }
        }

        Ok(ParseOutput::Object(map))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_fixture(input: &str, expected_json: &str) {
        let expected: serde_json::Value = serde_json::from_str(expected_json).unwrap();
        let result = ProcPidFdinfoParser.parse(input, false).unwrap();
        let got = serde_json::to_value(result).unwrap();
        assert_eq!(got, expected);
    }

    #[test]
    fn test_proc_pid_fdinfo() {
        parse_fixture(
            include_str!("../../../../tests/fixtures/linux-proc/pid_fdinfo"),
            include_str!("../../../../tests/fixtures/linux-proc/pid_fdinfo.json"),
        );
    }

    #[test]
    fn test_proc_pid_fdinfo_epoll() {
        parse_fixture(
            include_str!("../../../../tests/fixtures/linux-proc/pid_fdinfo_epoll"),
            include_str!("../../../../tests/fixtures/linux-proc/pid_fdinfo_epoll.json"),
        );
    }

    #[test]
    fn test_proc_pid_fdinfo_inotify() {
        parse_fixture(
            include_str!("../../../../tests/fixtures/linux-proc/pid_fdinfo_inotify"),
            include_str!("../../../../tests/fixtures/linux-proc/pid_fdinfo_inotify.json"),
        );
    }

    #[test]
    fn test_proc_pid_fdinfo_fanotify() {
        parse_fixture(
            include_str!("../../../../tests/fixtures/linux-proc/pid_fdinfo_fanotify"),
            include_str!("../../../../tests/fixtures/linux-proc/pid_fdinfo_fanotify.json"),
        );
    }

    #[test]
    fn test_proc_pid_fdinfo_timerfd() {
        parse_fixture(
            include_str!("../../../../tests/fixtures/linux-proc/pid_fdinfo_timerfd"),
            include_str!("../../../../tests/fixtures/linux-proc/pid_fdinfo_timerfd.json"),
        );
    }

    #[test]
    fn test_proc_pid_fdinfo_dma() {
        parse_fixture(
            include_str!("../../../../tests/fixtures/linux-proc/pid_fdinfo_dma"),
            include_str!("../../../../tests/fixtures/linux-proc/pid_fdinfo_dma.json"),
        );
    }
}
