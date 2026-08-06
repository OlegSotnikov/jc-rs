//! Parser for `/proc/<pid>/smaps`

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use regex::Regex;
use serde_json::{Map, Value};
use std::sync::LazyLock;

// Compiled once for the process. Each of these sat inside a function the
// parser calls per line or per record, so the pattern was rebuilt from
// source for every one of them.
static RE_MAP_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^([0-9a-f]{8,16})-([0-9a-f]{8,16})\s([rwxsp\-]{4})\s([0-9a-f]{8,9})\s([0-9a-f]{2}):([0-9a-f]{2})\s(\d+)\s+(.*)").expect("RE_MAP_RE is a valid pattern")
});

pub struct ProcPidSmapsParser;

static INFO: ParserInfo = ParserInfo {
    name: "proc_pid_smaps",
    argument: "--proc-pid-smaps",
    version: "1.0.0",
    description: "Converts `/proc/<pid>/smaps` file to JSON",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[Platform::Linux],
    tags: &[Tag::File, Tag::Slurpable],
    magic_commands: &[],
    streaming: false,
    hidden: true,
    deprecated: false,
};

static PROC_PID_SMAPS_PARSER: ProcPidSmapsParser = ProcPidSmapsParser;

inventory::submit! { ParserEntry::new(&PROC_PID_SMAPS_PARSER) }

fn perm_to_name(c: char) -> Option<&'static str> {
    match c {
        'r' => Some("read"),
        'w' => Some("write"),
        'x' => Some("execute"),
        's' => Some("shared"),
        'p' => Some("private"),
        '-' => None,
        _ => None,
    }
}

fn vmflag_pretty(flag: &str) -> String {
    match flag {
        "rd" => "readable".to_string(),
        "wr" => "writeable".to_string(),
        "ex" => "executable".to_string(),
        "sh" => "shared".to_string(),
        "mr" => "may read".to_string(),
        "mw" => "may write".to_string(),
        "me" => "may execute".to_string(),
        "ms" => "may share".to_string(),
        "mp" => "MPX-specific VMA".to_string(),
        "gd" => "stack segment growns down".to_string(),
        "pf" => "pure PFN range".to_string(),
        "dw" => "disabled write to the mapped file".to_string(),
        "lo" => "pages are locked in memory".to_string(),
        "io" => "memory mapped I/O area".to_string(),
        "sr" => "sequential read advise provided".to_string(),
        "rr" => "random read advise provided".to_string(),
        "dc" => "do not copy area on fork".to_string(),
        "de" => "do not expand area on remapping".to_string(),
        "ac" => "area is accountable".to_string(),
        "nr" => "swap space is not reserved for the area".to_string(),
        "ht" => "area uses huge tlb pages".to_string(),
        "ar" => "architecture specific flag".to_string(),
        "dd" => "do not include area into core dump".to_string(),
        "sd" => "soft-dirty flag".to_string(),
        "mm" => "mixed map area".to_string(),
        "hg" => "huge page advise flag".to_string(),
        "nh" => "no-huge page advise flag".to_string(),
        "mg" => "mergable advise flag".to_string(),
        other => other.to_string(),
    }
}

impl Parser for ProcPidSmapsParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Array(vec![]));
        }

        let map_re = &*RE_MAP_RE;

        let reserved_keys = [
            "start", "end", "perms", "offset", "maj", "min", "pathname", "VmFlags",
        ];

        let mut entries: Vec<Map<String, Value>> = Vec::new();
        let mut current: Option<Map<String, Value>> = None;

        for line in input.lines() {
            if let Some(caps) = map_re.captures(line) {
                // Save previous entry
                if let Some(entry) = current.take() {
                    entries.push(entry);
                }

                let mut map: Map<String, Value> = Map::new();
                map.insert("start".to_string(), Value::String(caps[1].to_string()));
                map.insert("end".to_string(), Value::String(caps[2].to_string()));

                let perms: Vec<Value> = caps[3]
                    .chars()
                    .filter_map(|c| perm_to_name(c).map(|s| Value::String(s.to_string())))
                    .collect();
                map.insert("perms".to_string(), Value::Array(perms));

                map.insert("offset".to_string(), Value::String(caps[4].to_string()));
                map.insert("maj".to_string(), Value::String(caps[5].to_string()));
                map.insert("min".to_string(), Value::String(caps[6].to_string()));

                let inode: u64 = caps[7].parse().unwrap_or(0);
                map.insert("inode".to_string(), Value::Number(inode.into()));

                let pathname = caps[8].trim().to_string();
                map.insert("pathname".to_string(), Value::String(pathname));

                current = Some(map);
            } else if let Some(ref mut map) = current {
                // Key: Value line
                if let Some(colon_pos) = line.find(':') {
                    let key = line[..colon_pos].trim().to_string();
                    let val = line[colon_pos + 1..].trim().to_string();

                    if key == "VmFlags" {
                        let flags: Vec<&str> = val.split_whitespace().collect();
                        let flags_arr: Vec<Value> =
                            flags.iter().map(|s| Value::String(s.to_string())).collect();
                        let pretty_arr: Vec<Value> = flags
                            .iter()
                            .map(|s| Value::String(vmflag_pretty(s).to_string()))
                            .collect();
                        map.insert("VmFlags".to_string(), Value::Array(flags_arr));
                        map.insert("VmFlags_pretty".to_string(), Value::Array(pretty_arr));
                    } else if reserved_keys.contains(&key.as_str()) {
                        map.insert(key, Value::String(val));
                    } else {
                        // Try to parse as integer (strip " kB" suffix)
                        let stripped = val.strip_suffix(" kB").unwrap_or(&val);
                        if let Ok(i) = stripped.trim().parse::<i64>() {
                            map.insert(key, Value::Number(i.into()));
                        } else {
                            map.insert(key, Value::String(val));
                        }
                    }
                }
            }
        }

        if let Some(entry) = current.take() {
            entries.push(entry);
        }

        Ok(ParseOutput::Array(entries))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proc_pid_smaps() {
        let input = include_str!("../../../../tests/fixtures/linux-proc/pid_smaps");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/linux-proc/pid_smaps.json"
        ))
        .unwrap();
        let parser = ProcPidSmapsParser;
        let result = parser.parse(input, false).unwrap();
        let got = serde_json::to_value(result).unwrap();
        assert_eq!(got, expected);
    }
}
