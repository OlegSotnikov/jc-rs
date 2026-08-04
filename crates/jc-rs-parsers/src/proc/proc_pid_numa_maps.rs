//! Parser for `/proc/<pid>/numa_maps`

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};

pub struct ProcPidNumaMapsParser;

static INFO: ParserInfo = ParserInfo {
    name: "proc_pid_numa_maps",
    argument: "--proc-pid-numa-maps",
    version: "1.0.0",
    description: "Converts `/proc/<pid>/numa_maps` file to JSON",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[Platform::Linux],
    tags: &[Tag::File, Tag::Slurpable],
    magic_commands: &[],
    streaming: false,
    hidden: true,
    deprecated: false,
};

static PROC_PID_NUMA_MAPS_PARSER: ProcPidNumaMapsParser = ProcPidNumaMapsParser;

inventory::submit! { ParserEntry::new(&PROC_PID_NUMA_MAPS_PARSER) }

impl Parser for ProcPidNumaMapsParser {
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
            let words: Vec<&str> = line.split_whitespace().collect();
            if words.len() < 2 {
                continue;
            }

            map.insert("address".to_string(), Value::String(words[0].to_string()));
            map.insert("policy".to_string(), Value::String(words[1].to_string()));

            let mut options: Vec<Value> = Vec::new();

            for word in &words[2..] {
                if let Some((key, val)) = word.split_once('=') {
                    // Try to convert value to integer
                    if let Ok(i) = val.parse::<i64>() {
                        map.insert(key.to_string(), Value::Number(i.into()));
                    } else {
                        map.insert(key.to_string(), Value::String(val.to_string()));
                    }
                } else {
                    options.push(Value::String(word.to_string()));
                }
            }

            if !options.is_empty() {
                map.insert("options".to_string(), Value::Array(options));
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
    fn test_proc_pid_numa_maps() {
        let input = include_str!("../../../../tests/fixtures/linux-proc/pid_numa_maps");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/linux-proc/pid_numa_maps.json"
        ))
        .unwrap();
        let parser = ProcPidNumaMapsParser;
        let result = parser.parse(input, false).unwrap();
        let got = serde_json::to_value(result).unwrap();
        assert_eq!(got, expected);
    }
}
