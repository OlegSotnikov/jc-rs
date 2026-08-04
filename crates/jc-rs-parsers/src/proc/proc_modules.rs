//! Parser for `/proc/modules`

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};

pub struct ProcModulesParser;

static INFO: ParserInfo = ParserInfo {
    name: "proc_modules",
    argument: "--proc-modules",
    version: "1.0.0",
    description: "Converts `/proc/modules` file to JSON",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[Platform::Linux],
    tags: &[Tag::File, Tag::Slurpable],
    magic_commands: &["/proc/modules"],
    streaming: false,
    hidden: true,
    deprecated: false,
};

static PROC_MODULES_PARSER: ProcModulesParser = ProcModulesParser;

inventory::submit! { ParserEntry::new(&PROC_MODULES_PARSER) }

impl Parser for ProcModulesParser {
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
            if parts.len() < 6 {
                continue;
            }

            let module = parts[0];
            let size: i64 = parts[1].parse().unwrap_or(0);
            let used: i64 = parts[2].parse().unwrap_or(0);

            // used_by field: parts[3] can be "-" or comma-separated list like "mod1,mod2,"
            let used_by: Vec<Value> = if parts[3] == "-" {
                vec![]
            } else {
                parts[3]
                    .split(',')
                    .filter(|s| !s.is_empty())
                    .map(|s| Value::String(s.to_string()))
                    .collect()
            };

            let status = parts[4];
            let location = parts[5];

            let mut entry = Map::new();
            entry.insert("module".to_string(), Value::String(module.to_string()));
            entry.insert("size".to_string(), Value::Number(size.into()));
            entry.insert("used".to_string(), Value::Number(used.into()));
            entry.insert("used_by".to_string(), Value::Array(used_by));
            entry.insert("status".to_string(), Value::String(status.to_string()));
            entry.insert("location".to_string(), Value::String(location.to_string()));

            entries.push(entry);
        }

        Ok(ParseOutput::Array(entries))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proc_modules() {
        let input = include_str!("../../../../tests/fixtures/linux-proc/modules");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/linux-proc/modules.json"
        ))
        .unwrap();
        let parser = ProcModulesParser;
        let result = parser.parse(input, false).unwrap();
        let got = serde_json::to_value(result).unwrap();
        assert_eq!(got, expected);
    }
}
