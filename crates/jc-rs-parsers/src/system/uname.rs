//! Parser for `uname -a` command output.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};

pub struct UnameParser;

static INFO: ParserInfo = ParserInfo {
    name: "uname",
    argument: "--uname",
    version: "1.8.0",
    description: "Converts `uname -a` command output to JSON",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[Platform::Linux, Platform::Darwin, Platform::FreeBSD],
    tags: &[Tag::Command, Tag::Slurpable],
    magic_commands: &["uname"],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static UNAME_PARSER: UnameParser = UnameParser;

inventory::submit! {
    ParserEntry::new(&UNAME_PARSER)
}

impl Parser for UnameParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        let record = parse_uname(input.trim())?;
        Ok(ParseOutput::Object(record))
    }
}

fn parse_uname(input: &str) -> Result<Map<String, Value>, ParseError> {
    if input.is_empty() {
        return Err(ParseError::InvalidInput("uname input is empty".to_string()));
    }

    let mut tokens: Vec<String> = input.split_whitespace().map(|s| s.to_string()).collect();

    if tokens.len() < 3 {
        return Err(ParseError::InvalidInput(format!(
            "uname: too few fields: {}",
            input
        )));
    }

    let mut record = Map::new();

    // Check for macOS/FreeBSD format
    if tokens[0] == "Darwin" || tokens[0] == "FreeBSD" {
        if tokens.len() < 5 {
            return Err(ParseError::InvalidInput(
                "Could not parse uname output. Make sure to use \"uname -a\".".to_string(),
            ));
        }
        let machine = tokens.pop().unwrap();
        let kernel_name = tokens.remove(0);
        let node_name = tokens.remove(0);
        let kernel_release = tokens.remove(0);
        let kernel_version = tokens.join(" ");

        record.insert("kernel_name".to_string(), Value::String(kernel_name));
        record.insert("node_name".to_string(), Value::String(node_name));
        record.insert("kernel_release".to_string(), Value::String(kernel_release));
        record.insert("kernel_version".to_string(), Value::String(kernel_version));
        record.insert("machine".to_string(), Value::String(machine));
    } else {
        // Linux format:
        // Fixup: if last 3 tokens (before OS) are all distinct → they are not the 3 arch fields
        // (machine, processor, hardware_platform). In that case, insert "unknown" twice.
        if tokens.len() >= 4 {
            let n = tokens.len();
            let set: std::collections::HashSet<&str> = [
                tokens[n - 2].as_str(),
                tokens[n - 3].as_str(),
                tokens[n - 4].as_str(),
            ]
            .iter()
            .cloned()
            .collect();
            if set.len() > 2 {
                // All 3 are different: no arch fields present, insert "unknown" for processor and hardware_platform
                let os_pos = tokens.len() - 1;
                tokens.insert(os_pos, "unknown".to_string());
                tokens.insert(os_pos, "unknown".to_string());
            }
        }

        // Now split: first 3 are kernel_name, node_name, kernel_release
        if tokens.len() < 3 {
            return Err(ParseError::InvalidInput(
                "Could not parse uname output. Make sure to use \"uname -a\".".to_string(),
            ));
        }

        let kernel_name = tokens.remove(0);
        let node_name = tokens.remove(0);
        let kernel_release = tokens.remove(0);

        // Remaining: ... kernel_version... machine processor hardware_platform os
        // rsplit: take last 4 tokens
        let n = tokens.len();
        if n < 4 {
            // Minimal: just remaining is kernel_version
            record.insert("kernel_name".to_string(), Value::String(kernel_name));
            record.insert("node_name".to_string(), Value::String(node_name));
            record.insert("kernel_release".to_string(), Value::String(kernel_release));
            record.insert(
                "kernel_version".to_string(),
                Value::String(tokens.join(" ")),
            );
            record.insert("machine".to_string(), Value::String(String::new()));
            record.insert("processor".to_string(), Value::String(String::new()));
            record.insert(
                "hardware_platform".to_string(),
                Value::String(String::new()),
            );
            record.insert("operating_system".to_string(), Value::String(String::new()));
            return Ok(record);
        }

        let operating_system = tokens.remove(n - 1);
        let processor = tokens.remove(n - 2);
        let hardware_platform = tokens.remove(n - 3);
        let machine = tokens.remove(n - 4);
        let kernel_version = tokens.join(" ");

        record.insert("kernel_name".to_string(), Value::String(kernel_name));
        record.insert("node_name".to_string(), Value::String(node_name));
        record.insert("kernel_release".to_string(), Value::String(kernel_release));
        record.insert(
            "operating_system".to_string(),
            Value::String(operating_system),
        );
        record.insert(
            "hardware_platform".to_string(),
            Value::String(hardware_platform),
        );
        record.insert("processor".to_string(), Value::String(processor));
        record.insert("machine".to_string(), Value::String(machine));
        record.insert("kernel_version".to_string(), Value::String(kernel_version));
    }

    Ok(record)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uname_generic() {
        let input = include_str!("../../../../tests/fixtures/generic/uname-a.out");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/generic/uname-a.json"
        ))
        .unwrap();

        let parser = UnameParser;
        let result = parser.parse(input, false).unwrap();
        let result_value: serde_json::Value = serde_json::to_value(result).unwrap();
        assert_eq!(result_value, expected);
    }

    #[test]
    fn test_uname_different_proc() {
        let input = include_str!("../../../../tests/fixtures/generic/uname-a-different-proc.out");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/generic/uname-a-different-proc.json"
        ))
        .unwrap();

        let parser = UnameParser;
        let result = parser.parse(input, false).unwrap();
        let result_value: serde_json::Value = serde_json::to_value(result).unwrap();
        assert_eq!(result_value, expected);
    }
}
