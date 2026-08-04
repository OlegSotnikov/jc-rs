//! Parser for `/proc/cpuinfo`

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};

pub struct ProcCpuinfoParser;

static INFO: ParserInfo = ParserInfo {
    name: "proc_cpuinfo",
    argument: "--proc-cpuinfo",
    version: "1.0.0",
    description: "Converts `/proc/cpuinfo` file to JSON",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[Platform::Linux],
    tags: &[Tag::File, Tag::Slurpable],
    magic_commands: &["/proc/cpuinfo"],
    streaming: false,
    hidden: true,
    deprecated: false,
};

static PROC_CPUINFO_PARSER: ProcCpuinfoParser = ProcCpuinfoParser;

inventory::submit! { ParserEntry::new(&PROC_CPUINFO_PARSER) }

impl Parser for ProcCpuinfoParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Array(vec![]));
        }

        let mut entries: Vec<Map<String, Value>> = Vec::new();
        let mut current: Map<String, Value> = Map::new();

        for line in input.lines() {
            if line.trim().is_empty() {
                continue;
            }

            // Start a new CPU entry when we see "processor"
            if line.starts_with("processor") && !current.is_empty() {
                entries.push(process_cpu_entry(current));
                current = Map::new();
            }

            if let Some(colon_pos) = line.find(':') {
                let key = line[..colon_pos].trim().to_string();
                let val = line[colon_pos + 1..].trim().to_string();
                current.insert(key, Value::String(val));
            }
        }

        if !current.is_empty() {
            entries.push(process_cpu_entry(current));
        }

        Ok(ParseOutput::Array(entries))
    }
}

fn process_cpu_entry(raw: Map<String, Value>) -> Map<String, Value> {
    let mut out: Map<String, Value> = Map::new();

    for (key, val) in &raw {
        let str_val = match val {
            Value::String(s) => s.clone(),
            _ => continue,
        };

        // Handle special array fields
        if key == "flags" || key == "bugs" {
            if str_val.is_empty() {
                out.insert(key.clone(), Value::Array(vec![]));
            } else {
                let arr: Vec<Value> = str_val
                    .split_whitespace()
                    .map(|s| Value::String(s.to_string()))
                    .collect();
                out.insert(key.clone(), Value::Array(arr));
            }
            continue;
        }

        // Empty string -> null
        if str_val.is_empty() {
            out.insert(key.clone(), Value::Null);
            continue;
        }

        // Try yes/no -> bool
        if str_val == "yes" {
            out.insert(key.clone(), Value::Bool(true));
            continue;
        }
        if str_val == "no" {
            out.insert(key.clone(), Value::Bool(false));
            continue;
        }

        // Try int
        if let Ok(i) = str_val.parse::<i64>() {
            out.insert(key.clone(), Value::Number(i.into()));
            continue;
        }

        // Try float if there's a dot
        if str_val.contains('.')
            && let Ok(f) = str_val.parse::<f64>()
        {
            let n = serde_json::Number::from_f64(f).unwrap_or_else(|| 0.into());
            out.insert(key.clone(), Value::Number(n));
            continue;
        }

        // Keep as string
        out.insert(key.clone(), Value::String(str_val));
    }

    // Derived fields from "address sizes"
    if let Some(Value::String(addr)) = raw.get("address sizes") {
        let parts: Vec<&str> = addr.split_whitespace().collect();
        if parts.len() >= 4 {
            if let Ok(phy) = parts[0].parse::<i64>() {
                out.insert(
                    "address_size_physical".to_string(),
                    Value::Number(phy.into()),
                );
            }
            if let Ok(virt) = parts[3].parse::<i64>() {
                out.insert(
                    "address_size_virtual".to_string(),
                    Value::Number(virt.into()),
                );
            }
        }
    }

    // Derived fields from "cache size"
    if let Some(Value::String(cache)) = raw.get("cache size") {
        let parts: Vec<&str> = cache.split_whitespace().collect();
        if parts.len() >= 2 {
            if let Ok(num) = parts[0].parse::<i64>() {
                out.insert("cache_size_num".to_string(), Value::Number(num.into()));
            }
            out.insert(
                "cache_size_unit".to_string(),
                Value::String(parts[1].to_string()),
            );
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proc_cpuinfo() {
        let input = include_str!("../../../../tests/fixtures/linux-proc/cpuinfo");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/linux-proc/cpuinfo.json"
        ))
        .unwrap();
        let parser = ProcCpuinfoParser;
        let result = parser.parse(input, false).unwrap();
        let got = serde_json::to_value(result).unwrap();
        assert_eq!(got, expected);
    }

    #[test]
    fn test_proc_cpuinfo2() {
        let input = include_str!("../../../../tests/fixtures/linux-proc/cpuinfo2");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/linux-proc/cpuinfo2.json"
        ))
        .unwrap();
        let parser = ProcCpuinfoParser;
        let result = parser.parse(input, false).unwrap();
        let got = serde_json::to_value(result).unwrap();
        assert_eq!(got, expected);
    }
}
