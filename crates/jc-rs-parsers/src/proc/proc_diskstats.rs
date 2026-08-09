//! Parser for `/proc/diskstats`

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};

pub struct ProcDiskstatsParser;

static INFO: ParserInfo = ParserInfo {
    name: "proc_diskstats",
    argument: "--proc-diskstats",
    version: "1.0.0",
    description: "Converts `/proc/diskstats` file to JSON",
    author: "jc-rs contributors",
    author_email: "os@g1sw.com",
    compatible: &[Platform::Linux],
    tags: &[Tag::File, Tag::Slurpable],
    magic_commands: &["/proc/diskstats"],
    streaming: false,
    hidden: true,
    deprecated: false,
};

static PROC_DISKSTATS_PARSER: ProcDiskstatsParser = ProcDiskstatsParser;
inventory::submit! { ParserEntry::new(&PROC_DISKSTATS_PARSER) }

const FIELD_NAMES: &[&str] = &[
    "maj",
    "min",
    "device",
    "reads_completed",
    "reads_merged",
    "sectors_read",
    "read_time_ms",
    "writes_completed",
    "writes_merged",
    "sectors_written",
    "write_time_ms",
    "io_in_progress",
    "io_time_ms",
    "weighted_io_time_ms",
    "discards_completed_successfully",
    "discards_merged",
    "sectors_discarded",
    "discarding_time_ms",
    "flush_requests_completed_successfully",
    "flushing_time_ms",
];

impl Parser for ProcDiskstatsParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Array(vec![]));
        }

        let mut entries: Vec<Map<String, Value>> = Vec::new();

        for line in input.lines() {
            if line.trim().is_empty() {
                continue;
            }

            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 3 {
                continue;
            }

            let mut entry = Map::new();
            for (i, &field) in FIELD_NAMES.iter().enumerate() {
                if i >= parts.len() {
                    break;
                }
                if field == "device" {
                    entry.insert(field.to_string(), Value::String(parts[i].to_string()));
                } else if let Ok(v) = parts[i].parse::<i64>() {
                    entry.insert(field.to_string(), Value::Number(v.into()));
                }
            }
            entries.push(entry);
        }

        Ok(ParseOutput::Array(entries))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proc_diskstats() {
        let input = include_str!("../../../../tests/fixtures/linux-proc/diskstats");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/linux-proc/diskstats.json"
        ))
        .unwrap();
        let parser = ProcDiskstatsParser;
        let result = parser.parse(input, false).unwrap();
        let got = serde_json::to_value(result).unwrap();
        assert_eq!(got, expected);
    }
}
