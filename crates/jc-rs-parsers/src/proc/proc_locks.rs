//! Parser for `/proc/locks`

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};

pub struct ProcLocksParser;

static INFO: ParserInfo = ParserInfo {
    name: "proc_locks",
    argument: "--proc-locks",
    version: "1.0.0",
    description: "Converts `/proc/locks` file to JSON",
    author: "jc-rs contributors",
    author_email: "os@g1sw.com",
    compatible: &[Platform::Linux],
    tags: &[Tag::File, Tag::Slurpable],
    magic_commands: &["/proc/locks"],
    streaming: false,
    hidden: true,
    deprecated: false,
};

static PROC_LOCKS_PARSER: ProcLocksParser = ProcLocksParser;

inventory::submit! { ParserEntry::new(&PROC_LOCKS_PARSER) }

impl Parser for ProcLocksParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Array(vec![]));
        }

        let mut entries = Vec::new();

        for line in input.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 8 {
                continue;
            }

            // id: first field with trailing ':' stripped
            let id_str = parts[0].trim_end_matches(':');
            let id: i64 = id_str.parse().unwrap_or(0);

            let class = parts[1];
            let lock_type = parts[2];
            let access = parts[3];
            let pid: i64 = parts[4].parse().unwrap_or(0);

            // device field: "00:19:812" or "fd:00:264967"
            let dev_parts: Vec<&str> = parts[5].split(':').collect();
            let maj = dev_parts.first().copied().unwrap_or("");
            let min = dev_parts.get(1).copied().unwrap_or("");
            let inode: i64 = dev_parts.get(2).and_then(|s| s.parse().ok()).unwrap_or(0);

            let start = parts[6];
            let end = parts[7];

            let mut entry = Map::new();
            entry.insert("id".to_string(), Value::Number(id.into()));
            entry.insert("class".to_string(), Value::String(class.to_string()));
            entry.insert("type".to_string(), Value::String(lock_type.to_string()));
            entry.insert("access".to_string(), Value::String(access.to_string()));
            entry.insert("pid".to_string(), Value::Number(pid.into()));
            entry.insert("maj".to_string(), Value::String(maj.to_string()));
            entry.insert("min".to_string(), Value::String(min.to_string()));
            entry.insert("inode".to_string(), Value::Number(inode.into()));
            entry.insert("start".to_string(), Value::String(start.to_string()));
            entry.insert("end".to_string(), Value::String(end.to_string()));

            entries.push(entry);
        }

        Ok(ParseOutput::Array(entries))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proc_locks() {
        let input = include_str!("../../../../tests/fixtures/linux-proc/locks");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/linux-proc/locks.json"
        ))
        .unwrap();
        let parser = ProcLocksParser;
        let result = parser.parse(input, false).unwrap();
        let got = serde_json::to_value(result).unwrap();
        assert_eq!(got, expected);
    }
}
