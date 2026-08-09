//! Parser for `/proc/<pid>/maps`

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};

pub struct ProcPidMapsParser;

static INFO: ParserInfo = ParserInfo {
    name: "proc_pid_maps",
    argument: "--proc-pid-maps",
    version: "1.0.0",
    description: "Converts `/proc/<pid>/maps` file to JSON",
    author: "jc-rs contributors",
    author_email: "os@g1sw.com",
    compatible: &[Platform::Linux],
    tags: &[Tag::File, Tag::Slurpable],
    magic_commands: &[],
    streaming: false,
    hidden: true,
    deprecated: false,
};

static PROC_PID_MAPS_PARSER: ProcPidMapsParser = ProcPidMapsParser;

inventory::submit! { ParserEntry::new(&PROC_PID_MAPS_PARSER) }

fn perm_to_name(c: char) -> Option<&'static str> {
    match c {
        'r' => Some("read"),
        'w' => Some("write"),
        'x' => Some("execute"),
        's' => Some("shared"),
        'p' => Some("private"),
        '-' => None,
        _ => None,
    }
}

impl Parser for ProcPidMapsParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Array(vec![]));
        }

        let mut entries: Vec<Map<String, Value>> = Vec::new();

        for line in input.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            let mut map: Map<String, Value> = Map::new();

            // Split into at most 6 parts: address perms offset dev inode [pathname]
            let parts: Vec<&str> = line.splitn(6, char::is_whitespace).collect();
            if parts.len() < 5 {
                continue;
            }

            // address -> start and end
            if let Some((start, end)) = parts[0].split_once('-') {
                map.insert("start".to_string(), Value::String(start.to_string()));
                map.insert("end".to_string(), Value::String(end.to_string()));
            }

            // perms
            let perms: Vec<Value> = parts[1]
                .chars()
                .filter_map(|c| perm_to_name(c).map(|s| Value::String(s.to_string())))
                .collect();
            map.insert("perms".to_string(), Value::Array(perms));

            // offset
            map.insert("offset".to_string(), Value::String(parts[2].to_string()));

            // dev -> maj and min
            if let Some((maj, min)) = parts[3].split_once(':') {
                map.insert("maj".to_string(), Value::String(maj.to_string()));
                map.insert("min".to_string(), Value::String(min.to_string()));
            }

            // inode
            if let Ok(inode) = parts[4].trim().parse::<u64>() {
                map.insert("inode".to_string(), Value::Number(inode.into()));
            }

            // pathname (may be absent)
            if parts.len() > 5 {
                let pathname = parts[5].trim();
                if !pathname.is_empty() {
                    map.insert("pathname".to_string(), Value::String(pathname.to_string()));
                }
            }

            entries.push(map);
        }

        Ok(ParseOutput::Array(entries))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proc_pid_maps() {
        let input = include_str!("../../../../tests/fixtures/linux-proc/pid_maps");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/linux-proc/pid_maps.json"
        ))
        .unwrap();
        let parser = ProcPidMapsParser;
        let result = parser.parse(input, false).unwrap();
        let got = serde_json::to_value(result).unwrap();
        assert_eq!(got, expected);
    }
}
