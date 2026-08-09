//! Parser for `/proc/<pid>/mountinfo`

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};

pub struct ProcPidMountinfoParser;

static INFO: ParserInfo = ParserInfo {
    name: "proc_pid_mountinfo",
    argument: "--proc-pid-mountinfo",
    version: "1.0.0",
    description: "Converts `/proc/<pid>/mountinfo` file to JSON",
    author: "jc-rs contributors",
    author_email: "os@g1sw.com",
    compatible: &[Platform::Linux],
    tags: &[Tag::File, Tag::Slurpable],
    magic_commands: &[],
    streaming: false,
    hidden: true,
    deprecated: false,
};

static PROC_PID_MOUNTINFO_PARSER: ProcPidMountinfoParser = ProcPidMountinfoParser;

inventory::submit! { ParserEntry::new(&PROC_PID_MOUNTINFO_PARSER) }

impl Parser for ProcPidMountinfoParser {
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

            // Split on " - " to separate the two halves
            let (left, right) = match line.split_once(" - ") {
                Some(pair) => pair,
                None => continue,
            };

            let left_parts: Vec<&str> = left.split_whitespace().collect();
            if left_parts.len() < 6 {
                continue;
            }

            let mut map: Map<String, Value> = Map::new();

            // mount_id parent_id maj:min root mount_point mount_options [optional_fields...]
            map.insert(
                "mount_id".to_string(),
                Value::Number(left_parts[0].parse::<i64>().unwrap_or(0).into()),
            );
            map.insert(
                "parent_id".to_string(),
                Value::Number(left_parts[1].parse::<i64>().unwrap_or(0).into()),
            );

            // dev maj:min
            if let Some((maj, min)) = left_parts[2].split_once(':') {
                map.insert(
                    "maj".to_string(),
                    Value::Number(maj.parse::<i64>().unwrap_or(0).into()),
                );
                map.insert(
                    "min".to_string(),
                    Value::Number(min.parse::<i64>().unwrap_or(0).into()),
                );
            }

            map.insert("root".to_string(), Value::String(left_parts[3].to_string()));
            map.insert(
                "mount_point".to_string(),
                Value::String(left_parts[4].to_string()),
            );

            // mount_options -> split on comma
            let mount_options: Vec<Value> = left_parts[5]
                .split(',')
                .map(|s| Value::String(s.to_string()))
                .collect();
            map.insert("mount_options".to_string(), Value::Array(mount_options));

            // optional_fields (index 6+)
            let mut opt_fields: Map<String, Value> = Map::new();
            for part in &left_parts[6..] {
                if *part == "unbindable" {
                    opt_fields.insert("unbindable".to_string(), Value::Number(0.into()));
                } else if let Some((key, val)) = part.split_once(':') {
                    if let Ok(v) = val.parse::<i64>() {
                        opt_fields.insert(key.to_string(), Value::Number(v.into()));
                    } else {
                        opt_fields.insert(key.to_string(), Value::String(val.to_string()));
                    }
                }
            }
            map.insert("optional_fields".to_string(), Value::Object(opt_fields));

            // Right side: fs_type mount_source [super_options]
            let right_parts: Vec<&str> = right.split_whitespace().collect();
            if right_parts.len() < 2 {
                continue;
            }

            map.insert(
                "fs_type".to_string(),
                Value::String(right_parts[0].to_string()),
            );
            map.insert(
                "mount_source".to_string(),
                Value::String(right_parts[1].to_string()),
            );

            // super_options (index 2, if present)
            if right_parts.len() > 2 {
                let super_opts_str = right_parts[2].trim();
                if !super_opts_str.is_empty() {
                    let mut super_options: Vec<Value> = Vec::new();
                    let mut super_options_fields: Map<String, Value> = Map::new();

                    for part in super_opts_str.split(',') {
                        if let Some((key, val)) = part.split_once('=') {
                            // Strip size suffixes (k, K, m, M, etc.) and parse as integer
                            let num_str = val
                                .strip_suffix('k')
                                .or_else(|| val.strip_suffix('K'))
                                .or_else(|| val.strip_suffix('m'))
                                .or_else(|| val.strip_suffix('M'))
                                .unwrap_or(val);
                            if let Ok(v) = num_str.parse::<i64>() {
                                super_options_fields
                                    .insert(key.to_string(), Value::Number(v.into()));
                            } else {
                                super_options_fields.insert(key.to_string(), Value::Null);
                            }
                        } else {
                            super_options.push(Value::String(part.to_string()));
                        }
                    }

                    if !super_options.is_empty() {
                        map.insert("super_options".to_string(), Value::Array(super_options));
                    }
                    if !super_options_fields.is_empty() {
                        map.insert(
                            "super_options_fields".to_string(),
                            Value::Object(super_options_fields),
                        );
                    }
                }
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
    fn test_proc_pid_mountinfo() {
        let input = include_str!("../../../../tests/fixtures/linux-proc/pid_mountinfo");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/linux-proc/pid_mountinfo.json"
        ))
        .unwrap();
        let parser = ProcPidMountinfoParser;
        let result = parser.parse(input, false).unwrap();
        let got = serde_json::to_value(result).unwrap();
        assert_eq!(got, expected);
    }
}
