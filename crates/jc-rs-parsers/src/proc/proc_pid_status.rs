//! Parser for `/proc/<pid>/status`

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};

pub struct ProcPidStatusParser;

static INFO: ParserInfo = ParserInfo {
    name: "proc_pid_status",
    argument: "--proc-pid-status",
    version: "1.0.0",
    description: "Converts `/proc/<pid>/status` file to JSON",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[Platform::Linux],
    tags: &[Tag::File, Tag::Slurpable],
    magic_commands: &[],
    streaming: false,
    hidden: true,
    deprecated: false,
};

static PROC_PID_STATUS_PARSER: ProcPidStatusParser = ProcPidStatusParser;

inventory::submit! { ParserEntry::new(&PROC_PID_STATUS_PARSER) }

const INT_FIELDS: &[&str] = &[
    "Tgid",
    "Ngid",
    "Pid",
    "PPid",
    "TracerPid",
    "FDSize",
    "NStgid",
    "NSpid",
    "NSpgid",
    "NSsid",
    "VmPeak",
    "VmSize",
    "VmLck",
    "VmPin",
    "VmHWM",
    "VmRSS",
    "RssAnon",
    "RssFile",
    "RssShmem",
    "VmData",
    "VmStk",
    "VmExe",
    "VmLib",
    "VmPTE",
    "VmSwap",
    "HugetlbPages",
    "CoreDumping",
    "THP_enabled",
    "Threads",
    "NoNewPrivs",
    "Seccomp",
    "voluntary_ctxt_switches",
    "nonvoluntary_ctxt_switches",
];

impl Parser for ProcPidStatusParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Err(ParseError::Generic("Empty input".to_string()));
        }

        let mut map: Map<String, Value> = Map::new();

        for line in input.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            let colon_pos = match line.find(':') {
                Some(p) => p,
                None => continue,
            };

            let key = line[..colon_pos].trim().to_string();
            let val = line[colon_pos + 1..].trim().to_string();

            if key == "State" {
                // "S (sleeping)" -> State = "S", State_pretty = "sleeping"
                let parts: Vec<&str> = val.splitn(2, ' ').collect();
                map.insert("State".to_string(), Value::String(parts[0].to_string()));
                if parts.len() > 1 {
                    let pretty = parts[1].trim_matches(|c| c == '(' || c == ')');
                    map.insert(
                        "State_pretty".to_string(),
                        Value::String(pretty.to_string()),
                    );
                }
            } else if key == "Uid" || key == "Gid" {
                // Tab-separated ints -> array
                let arr: Vec<Value> = val
                    .split('\t')
                    .filter(|s| !s.is_empty())
                    .filter_map(|s| s.trim().parse::<i64>().ok())
                    .map(|i| Value::Number(i.into()))
                    .collect();
                map.insert(key, Value::Array(arr));
            } else if key == "SigQ" {
                // "0/15245" -> SigQ as string, SigQ_current and SigQ_limit as ints
                map.insert("SigQ".to_string(), Value::String(val.clone()));
                if let Some((current, limit)) = val.split_once('/') {
                    if let Ok(c) = current.trim().parse::<i64>() {
                        map.insert("SigQ_current".to_string(), Value::Number(c.into()));
                    }
                    if let Ok(l) = limit.trim().parse::<i64>() {
                        map.insert("SigQ_limit".to_string(), Value::Number(l.into()));
                    }
                }
            } else if key == "Cpus_allowed" || key == "Mems_allowed" {
                // Comma-separated -> array of strings
                let arr: Vec<Value> = val
                    .split(',')
                    .map(|s| Value::String(s.trim().to_string()))
                    .collect();
                map.insert(key, Value::Array(arr));
            } else if key == "Groups" {
                // Keep as string (may be empty or space-separated)
                map.insert(key, Value::String(val.to_string()));
            } else if INT_FIELDS.contains(&key.as_str()) {
                // Strip " kB" suffix, parse as int
                let stripped = val.strip_suffix(" kB").unwrap_or(&val);
                if let Ok(i) = stripped.trim().parse::<i64>() {
                    map.insert(key, Value::Number(i.into()));
                } else {
                    map.insert(key, Value::String(val));
                }
            } else {
                // Keep as string
                map.insert(key, Value::String(val));
            }
        }

        Ok(ParseOutput::Object(map))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proc_pid_status() {
        let input = include_str!("../../../../tests/fixtures/linux-proc/pid_status");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/linux-proc/pid_status.json"
        ))
        .unwrap();
        let parser = ProcPidStatusParser;
        let result = parser.parse(input, false).unwrap();
        let got = serde_json::to_value(result).unwrap();
        assert_eq!(got, expected);
    }
}
