//! Parser for `/proc/filesystems`

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};

pub struct ProcFilesystemsParser;

static INFO: ParserInfo = ParserInfo {
    name: "proc_filesystems",
    argument: "--proc-filesystems",
    version: "1.0.0",
    description: "Converts `/proc/filesystems` file to JSON",
    author: "jc-rs contributors",
    author_email: "os@g1sw.com",
    compatible: &[Platform::Linux],
    tags: &[Tag::File, Tag::Slurpable],
    magic_commands: &["/proc/filesystems"],
    streaming: false,
    hidden: true,
    deprecated: false,
};

static PROC_FILESYSTEMS_PARSER: ProcFilesystemsParser = ProcFilesystemsParser;

inventory::submit! { ParserEntry::new(&PROC_FILESYSTEMS_PARSER) }

impl Parser for ProcFilesystemsParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Array(vec![]));
        }

        let mut entries = Vec::new();

        for line in input.lines() {
            if line.trim().is_empty() {
                continue;
            }

            // Format: "nodev\tname" or "\tname"
            let nodev = line.starts_with("nodev");
            let fs_name = if nodev {
                line.trim_start_matches("nodev").trim()
            } else {
                line.trim()
            };

            let mut entry = Map::new();
            entry.insert("filesystem".to_string(), Value::String(fs_name.to_string()));
            entry.insert("nodev".to_string(), Value::Bool(nodev));
            entries.push(entry);
        }

        Ok(ParseOutput::Array(entries))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proc_filesystems() {
        let input = include_str!("../../../../tests/fixtures/linux-proc/filesystems");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/linux-proc/filesystems.json"
        ))
        .unwrap();
        let parser = ProcFilesystemsParser;
        let result = parser.parse(input, false).unwrap();
        let got = serde_json::to_value(result).unwrap();
        assert_eq!(got, expected);
    }
}
