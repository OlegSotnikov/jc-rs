//! Parser for `/proc/<pid>/io`

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};

pub struct ProcPidIoParser;

static INFO: ParserInfo = ParserInfo {
    name: "proc_pid_io",
    argument: "--proc-pid-io",
    version: "1.0.0",
    description: "Converts `/proc/<pid>/io` file to JSON",
    author: "jc-rs contributors",
    author_email: "os@g1sw.com",
    compatible: &[Platform::Linux],
    tags: &[Tag::File, Tag::Slurpable],
    magic_commands: &["/proc/<pid>/io"],
    streaming: false,
    hidden: true,
    deprecated: false,
};

static INSTANCE: ProcPidIoParser = ProcPidIoParser;
inventory::submit! { ParserEntry::new(&INSTANCE) }

impl Parser for ProcPidIoParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Object(Map::new()));
        }

        let mut map = Map::new();

        for line in input.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            if let Some((key, val)) = line.split_once(": ") {
                let n: i64 = val.trim().parse().unwrap_or(0);
                map.insert(key.trim().to_string(), Value::Number(n.into()));
            }
        }

        Ok(ParseOutput::Object(map))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proc_pid_io() {
        let input = include_str!("../../../../tests/fixtures/linux-proc/pid_io");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/linux-proc/pid_io.json"
        ))
        .unwrap();
        let result = ProcPidIoParser.parse(input, false).unwrap();
        let got = serde_json::to_value(result).unwrap();
        assert_eq!(got, expected);
    }
}
