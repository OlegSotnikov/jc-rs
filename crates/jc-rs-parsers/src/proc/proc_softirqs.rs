//! Parser for `/proc/softirqs`

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};

pub struct ProcSoftirqsParser;

static INFO: ParserInfo = ParserInfo {
    name: "proc_softirqs",
    argument: "--proc-softirqs",
    version: "1.0.0",
    description: "Converts `/proc/softirqs` file to JSON",
    author: "jc-rs contributors",
    author_email: "os@g1sw.com",
    compatible: &[Platform::Linux],
    tags: &[Tag::File, Tag::Slurpable],
    magic_commands: &["/proc/softirqs"],
    streaming: false,
    hidden: true,
    deprecated: false,
};

static PROC_SOFTIRQS_PARSER: ProcSoftirqsParser = ProcSoftirqsParser;

inventory::submit! { ParserEntry::new(&PROC_SOFTIRQS_PARSER) }

impl Parser for ProcSoftirqsParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Array(vec![]));
        }

        let mut lines = input.lines();

        // First line has CPU headers
        let header_line = match lines.next() {
            Some(l) => l,
            None => return Ok(ParseOutput::Array(vec![])),
        };
        let cpu_names: Vec<&str> = header_line.split_whitespace().collect();

        let mut entries = Vec::new();

        for line in lines {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            let mut tokens: Vec<&str> = line.split_whitespace().collect();
            if tokens.is_empty() {
                continue;
            }

            // First token is the counter name (strip trailing ':')
            let counter = tokens.remove(0).trim_end_matches(':').to_string();

            let mut entry = Map::new();
            entry.insert("counter".to_string(), Value::String(counter));

            for (i, cpu_name) in cpu_names.iter().enumerate() {
                let val = tokens
                    .get(i)
                    .and_then(|t| t.parse::<i64>().ok())
                    .unwrap_or(0);
                entry.insert(cpu_name.to_string(), Value::Number(val.into()));
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
    fn test_proc_softirqs() {
        let input = include_str!("../../../../tests/fixtures/linux-proc/softirqs");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/linux-proc/softirqs.json"
        ))
        .unwrap();
        let parser = ProcSoftirqsParser;
        let result = parser.parse(input, false).unwrap();
        let got = serde_json::to_value(result).unwrap();
        assert_eq!(got, expected);
    }
}
