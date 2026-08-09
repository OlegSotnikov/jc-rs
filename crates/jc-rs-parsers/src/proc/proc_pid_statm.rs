//! Parser for `/proc/<pid>/statm`

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};

pub struct ProcPidStatmParser;

static INFO: ParserInfo = ParserInfo {
    name: "proc_pid_statm",
    argument: "--proc-pid-statm",
    version: "1.0.0",
    description: "Converts `/proc/<pid>/statm` file to JSON",
    author: "jc-rs contributors",
    author_email: "os@g1sw.com",
    compatible: &[Platform::Linux],
    tags: &[Tag::File, Tag::Slurpable],
    magic_commands: &[],
    streaming: false,
    hidden: true,
    deprecated: false,
};

static PROC_PID_STATM_PARSER: ProcPidStatmParser = ProcPidStatmParser;

inventory::submit! { ParserEntry::new(&PROC_PID_STATM_PARSER) }

impl Parser for ProcPidStatmParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        let field_names = ["size", "resident", "shared", "text", "lib", "data", "dt"];

        let parts: Vec<&str> = input.split_whitespace().collect();
        if parts.len() < 7 {
            return Err(ParseError::Generic(
                "Expected 7 fields in statm".to_string(),
            ));
        }

        let mut map: Map<String, Value> = Map::new();
        for (i, name) in field_names.iter().enumerate() {
            let val: i64 = parts[i]
                .parse()
                .map_err(|_| ParseError::Generic(format!("Invalid integer for {}", name)))?;
            map.insert(name.to_string(), Value::Number(val.into()));
        }

        Ok(ParseOutput::Object(map))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proc_pid_statm() {
        let input = include_str!("../../../../tests/fixtures/linux-proc/pid_statm");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/linux-proc/pid_statm.json"
        ))
        .unwrap();
        let parser = ProcPidStatmParser;
        let result = parser.parse(input, false).unwrap();
        let got = serde_json::to_value(result).unwrap();
        assert_eq!(got, expected);
    }
}
