//! Parser for `lsmod` command output.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use jc_rs_utils::simple_table_parse;
use serde_json::{Map, Value};

pub struct LsmodParser;

static INFO: ParserInfo = ParserInfo {
    name: "lsmod",
    argument: "--lsmod",
    version: "1.7.0",
    description: "Converts `lsmod` command output to JSON",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[Platform::Linux],
    tags: &[Tag::Command],
    magic_commands: &["lsmod"],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static LSMOD_PARSER: LsmodParser = LsmodParser;

inventory::submit! {
    ParserEntry::new(&LSMOD_PARSER)
}

impl Parser for LsmodParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Array(Vec::new()));
        }

        // Normalize header to lowercase
        let lines: Vec<&str> = input.lines().collect();
        if lines.is_empty() {
            return Ok(ParseOutput::Array(Vec::new()));
        }

        let header = lines[0].to_lowercase();
        let rest = lines[1..].join("\n");
        let normalized = format!("{}\n{}", header, rest);

        let rows = simple_table_parse(&normalized);

        let result = rows
            .into_iter()
            .map(|row| {
                let mut out = Map::new();

                if let Some(Value::String(s)) = row.get("module") {
                    out.insert("module".to_string(), Value::String(s.clone()));
                }

                if let Some(Value::String(s)) = row.get("size") {
                    let trimmed = s.trim();
                    if let Ok(n) = trimmed.parse::<i64>() {
                        out.insert("size".to_string(), Value::Number(n.into()));
                    } else {
                        out.insert("size".to_string(), Value::String(trimmed.to_string()));
                    }
                }

                if let Some(Value::String(s)) = row.get("used") {
                    let trimmed = s.trim();
                    if let Ok(n) = trimmed.parse::<i64>() {
                        out.insert("used".to_string(), Value::Number(n.into()));
                    } else {
                        out.insert("used".to_string(), Value::String(trimmed.to_string()));
                    }
                }

                // "by" column contains comma-separated module names
                if let Some(Value::String(s)) = row.get("by") {
                    let trimmed = s.trim();
                    if !trimmed.is_empty() {
                        let modules: Vec<Value> = trimmed
                            .split(',')
                            .map(|m| Value::String(m.trim().to_string()))
                            .collect();
                        out.insert("by".to_string(), Value::Array(modules));
                    }
                }

                out
            })
            .collect();

        Ok(ParseOutput::Array(result))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lsmod_basic() {
        let input = "Module                  Size  Used by\nipt_MASQUERADE         12678  1 \nnf_nat_masquerade_ipv4    13430  1 ipt_MASQUERADE\nllc                    14552  2 stp,bridge\n";
        let parser = LsmodParser;
        let result = parser.parse(input, false).unwrap();
        if let ParseOutput::Array(arr) = result {
            assert_eq!(arr.len(), 3);
            assert_eq!(
                arr[0].get("module"),
                Some(&Value::String("ipt_MASQUERADE".to_string()))
            );
            assert_eq!(arr[0].get("size"), Some(&Value::Number(12678.into())));
            assert_eq!(arr[0].get("used"), Some(&Value::Number(1.into())));
            assert!(arr[0].get("by").is_none());
            assert_eq!(
                arr[1].get("by"),
                Some(&Value::Array(vec![Value::String(
                    "ipt_MASQUERADE".to_string()
                )]))
            );
            assert_eq!(
                arr[2].get("by"),
                Some(&Value::Array(vec![
                    Value::String("stp".to_string()),
                    Value::String("bridge".to_string()),
                ]))
            );
        } else {
            panic!("Expected Array");
        }
    }

    #[test]
    fn test_lsmod_empty() {
        let parser = LsmodParser;
        let result = parser.parse("", false).unwrap();
        if let ParseOutput::Array(arr) = result {
            assert!(arr.is_empty());
        } else {
            panic!("Expected Array");
        }
    }
}
