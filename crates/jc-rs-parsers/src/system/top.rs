//! Parser for `top -b` command output.
//!
//! Requires batch mode (`-b`). Use with `-n` option to limit iterations.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::{LineParser, Parser, Record};
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use jc_rs_utils::{convert_size_to_int, convert_to_float, convert_to_int, sparse_table_parse};
use regex::Regex;
use serde_json::{Map, Value};
use std::sync::OnceLock;

pub struct TopParser;

static INFO: ParserInfo = ParserInfo {
    name: "top",
    argument: "--top",
    version: "1.3.0",
    description: "Converts `top -b` command output to JSON",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[Platform::Linux],
    tags: &[Tag::Command],
    magic_commands: &["top -b"],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static TOP_PARSER: TopParser = TopParser;

inventory::submit! {
    ParserEntry::new(&TOP_PARSER)
}

impl Parser for TopParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, quiet: bool) -> Result<ParseOutput, ParseError> {
        let rows = parse_top(input, quiet);
        Ok(ParseOutput::Array(rows))
    }
}

fn top_line_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"^(\d{1,2}:\d{2}(?::\d{2})?)\s+up\s+(.+?),\s*(\d+)\s+users?,\s*load\s+averages?:\s*([\d.]+)[,\s]+([\d.]+)[,\s]+([\d.]+)"
        ).unwrap()
    })
}

fn process_key_map(key: &str) -> Option<&'static str> {
    match key {
        "%CPU" => Some("percent_cpu"),
        "%MEM" => Some("percent_mem"),
        "CGNAME" => Some("control_group_name"),
        "CGROUPS" => Some("cgroups"),
        "CODE" => Some("code"),
        "COMMAND" => Some("command"),
        "DATA" => Some("data"),
        "ENVIRON" => Some("environment_variables"),
        "Flags" => Some("flags"),
        "GID" => Some("gid"),
        "GROUP" => Some("group"),
        "LXC" => Some("lxc_container_name"),
        "NI" => Some("nice"),
        "NU" => Some("numa_node"),
        "OOMa" => Some("out_of_mem_adjustment"),
        "OOMs" => Some("out_of_mem_score"),
        "P" => Some("last_used_processor"),
        "PGRP" => Some("pgrp"),
        "PID" => Some("pid"),
        "PPID" => Some("parent_pid"),
        "PR" => Some("priority"),
        "RES" => Some("resident_mem"),
        "RSan" => Some("resident_anon_mem"),
        "RSfd" => Some("resident_file_backed_mem"),
        "RSlk" => Some("resident_locked_mem"),
        "RSsh" => Some("resident_shared_mem"),
        "RUID" => Some("real_uid"),
        "RUSER" => Some("real_user"),
        "S" => Some("status"),
        "SHR" => Some("shared_mem"),
        "SID" => Some("session_id"),
        "SUID" => Some("saved_uid"),
        "SUPGIDS" => Some("supplementary_gids"),
        "SUPGRPS" => Some("supplementary_groups"),
        "SUSER" => Some("saved_user"),
        "SWAP" => Some("swap"),
        "TGID" => Some("thread_gid"),
        "TIME" => Some("time"),
        "TIME+" => Some("time_hundredths"),
        "TPGID" => Some("tty_process_gid"),
        "TTY" => Some("tty"),
        "UID" => Some("uid"),
        "USED" => Some("used"),
        "USER" => Some("user"),
        "VIRT" => Some("virtual_mem"),
        "WCHAN" => Some("sleeping_in_function"),
        "nDRT" => Some("dirty_pages_count"),
        "nMaj" => Some("major_page_fault_count"),
        "nMin" => Some("minor_page_fault_count"),
        "nTH" => Some("thread_count"),
        "nsIPC" => Some("ipc_namespace_inode"),
        "nsMNT" => Some("mount_namespace_inode"),
        "nsNET" => Some("net_namespace_inode"),
        "nsPID" => Some("pid_namespace_inode"),
        "nsUSER" => Some("user_namespace_inode"),
        "nsUTS" => Some("nts_namespace_inode"),
        "vMj" => Some("major_page_fault_count_delta"),
        "vMn" => Some("minor_page_fault_count_delta"),
        _ => None,
    }
}

fn map_status(s: &str) -> &'static str {
    match s {
        "D" => "uninterruptible sleep",
        "I" => "idle",
        "R" => "running",
        "S" => "sleeping",
        "T" => "stopped by job control signal",
        "t" => "stopped by debugger during trace",
        "Z" => "zombie",
        _ => "unknown",
    }
}

const BYTES_KEYS: &[&str] = &[
    "virtual_mem",
    "resident_mem",
    "shared_mem",
    "swap",
    "code",
    "data",
    "used",
];

const INT_KEYS: &[&str] = &[
    "pid",
    "priority",
    "nice",
    "parent_pid",
    "uid",
    "real_uid",
    "saved_uid",
    "gid",
    "pgrp",
    "tty_process_gid",
    "session_id",
    "thread_count",
    "last_used_processor",
    "major_page_fault_count",
    "minor_page_fault_count",
    "dirty_pages_count",
    "thread_gid",
    "major_page_fault_count_delta",
    "minor_page_fault_count_delta",
    "ipc_namespace_inode",
    "mount_namespace_inode",
    "net_namespace_inode",
    "pid_namespace_inode",
    "user_namespace_inode",
    "nts_namespace_inode",
    "numa_node",
    "out_of_mem_adjustment",
    "out_of_mem_score",
    "resident_anon_mem",
    "resident_file_backed_mem",
    "resident_locked_mem",
    "resident_shared_mem",
];

const FLOAT_KEYS: &[&str] = &[
    "percent_cpu",
    "percent_mem",
    "virtual_mem",
    "resident_mem",
    "shared_mem",
    "swap",
    "code",
    "data",
    "used",
];

/// `top` repeats a whole sample (header lines then a process table) and opens
/// the next one with `top - `. A sample is complete only when the next
/// one begins, or at end of input.
#[derive(Default)]
pub(crate) struct TopSession {
    item_obj: Map<String, Value>,
    process_table: bool,
    process_lines: Vec<String>,
}

impl TopSession {
    fn flush(&mut self) -> Option<Record> {
        if self.item_obj.is_empty() {
            return None;
        }
        if !self.process_lines.is_empty() {
            let table_str = self.process_lines.join("\n");
            let procs = parse_process_table(&table_str);
            self.item_obj
                .insert("processes".to_string(), Value::Array(procs));
            self.process_lines.clear();
        }
        self.process_table = false;
        Some(process_snapshot(std::mem::take(&mut self.item_obj)))
    }
}

impl LineParser for TopSession {
    fn parse_line(&mut self, line: &str, _quiet: bool) -> Result<Option<Record>, ParseError> {
        let mut finished = None;
        if let Some(rest) = line.strip_prefix("top - ") {
            // Flush previous snapshot
            finished = self.flush();

            // Parse "top - HH:MM:SS up ..." line
            // strip "top - "
            if let Some(caps) = top_line_re().captures(rest) {
                let time_str = caps[1].to_string();
                let uptime_str = caps[2].trim().to_string();
                let users_str = caps[3].to_string();
                let load1_str = caps[4].to_string();
                let load5_str = caps[5].to_string();
                let load15_str = caps[6].to_string();

                self.item_obj
                    .insert("time".to_string(), Value::String(time_str));
                self.item_obj
                    .insert("uptime".to_string(), Value::String(uptime_str));
                self.item_obj
                    .insert("users".to_string(), Value::String(users_str));
                self.item_obj
                    .insert("load_1m".to_string(), Value::String(load1_str));
                self.item_obj
                    .insert("load_5m".to_string(), Value::String(load5_str));
                self.item_obj
                    .insert("load_15m".to_string(), Value::String(load15_str));
            }
            // Hand back the sample this line closed, if any.
            return Ok(finished);
        }

        if line.starts_with("Tasks:") {
            // Tasks: 108 total,   2 running, 106 sleeping,   0 stopped,   0 zombie
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 10 {
                self.item_obj.insert(
                    "tasks_total".to_string(),
                    Value::String(parts[1].to_string()),
                );
                self.item_obj.insert(
                    "tasks_running".to_string(),
                    Value::String(parts[3].to_string()),
                );
                self.item_obj.insert(
                    "tasks_sleeping".to_string(),
                    Value::String(parts[5].to_string()),
                );
                self.item_obj.insert(
                    "tasks_stopped".to_string(),
                    Value::String(parts[7].to_string()),
                );
                self.item_obj.insert(
                    "tasks_zombie".to_string(),
                    Value::String(parts[9].to_string()),
                );
            }
            return Ok(None);
        }

        if line.starts_with("%Cpu(s):") {
            // %Cpu(s):  5.9 us,  5.9 sy,  0.0 ni, 88.2 id,  0.0 wa,  0.0 hi,  0.0 si,  0.0 st
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 16 {
                self.item_obj
                    .insert("cpu_user".to_string(), Value::String(parts[1].to_string()));
                self.item_obj
                    .insert("cpu_sys".to_string(), Value::String(parts[3].to_string()));
                self.item_obj
                    .insert("cpu_nice".to_string(), Value::String(parts[5].to_string()));
                self.item_obj
                    .insert("cpu_idle".to_string(), Value::String(parts[7].to_string()));
                self.item_obj
                    .insert("cpu_wait".to_string(), Value::String(parts[9].to_string()));
                self.item_obj.insert(
                    "cpu_hardware".to_string(),
                    Value::String(parts[11].to_string()),
                );
                self.item_obj.insert(
                    "cpu_software".to_string(),
                    Value::String(parts[13].to_string()),
                );
                self.item_obj.insert(
                    "cpu_steal".to_string(),
                    Value::String(parts[15].to_string()),
                );
            }
            return Ok(None);
        }

        // XiB Mem : N total, N free, N used, N buff/cache
        // Matches: "KiB Mem :", "MiB Mem :", "GiB Mem :"
        if line.len() > 1 && line[1..].starts_with("iB Mem") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 10 {
                self.item_obj
                    .insert("mem_unit".to_string(), Value::String(parts[0].to_string()));
                self.item_obj
                    .insert("mem_total".to_string(), Value::String(parts[3].to_string()));
                self.item_obj
                    .insert("mem_free".to_string(), Value::String(parts[5].to_string()));
                self.item_obj
                    .insert("mem_used".to_string(), Value::String(parts[7].to_string()));
                self.item_obj.insert(
                    "mem_buff_cache".to_string(),
                    Value::String(parts[9].to_string()),
                );
            }
            return Ok(None);
        }

        // XiB Swap: N total, N free, N used. N avail Mem
        if line.len() > 1 && line[1..].starts_with("iB Swap") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 9 {
                self.item_obj
                    .insert("swap_unit".to_string(), Value::String(parts[0].to_string()));
                self.item_obj.insert(
                    "swap_total".to_string(),
                    Value::String(parts[2].to_string()),
                );
                self.item_obj
                    .insert("swap_free".to_string(), Value::String(parts[4].to_string()));
                self.item_obj
                    .insert("swap_used".to_string(), Value::String(parts[6].to_string()));
                self.item_obj.insert(
                    "mem_available".to_string(),
                    Value::String(parts[8].to_string()),
                );
            }
            return Ok(None);
        }

        // Empty line signals start of process table
        if !self.process_table && line.trim().is_empty() {
            self.process_table = true;
            return Ok(None);
        }

        if self.process_table && !line.trim().is_empty() {
            self.process_lines.push(line.to_string());
        }
        Ok(finished)
    }

    fn finalize(&mut self, _quiet: bool) -> Result<Option<Record>, ParseError> {
        Ok(self.flush())
    }
}

pub fn parse_top(input: &str, _quiet: bool) -> Vec<Map<String, Value>> {
    let mut session = TopSession::default();
    let mut out: Vec<Map<String, Value>> = input
        .lines()
        .filter_map(|line| session.parse_line(line, true).ok().flatten())
        .collect();
    if let Ok(Some(last)) = session.finalize(true) {
        out.push(last);
    }
    out
}

/// Convert raw string values in a top snapshot to proper types.
fn process_snapshot(raw: Map<String, Value>) -> Map<String, Value> {
    let root_int_list = &[
        "uptime",
        "users",
        "tasks_total",
        "tasks_running",
        "tasks_sleeping",
        "tasks_stopped",
        "tasks_zombie",
    ];
    let root_float_list = &[
        "load_1m",
        "load_5m",
        "load_15m",
        "cpu_user",
        "cpu_sys",
        "cpu_nice",
        "cpu_idle",
        "cpu_wait",
        "cpu_hardware",
        "cpu_software",
        "cpu_steal",
        "mem_total",
        "mem_free",
        "mem_used",
        "mem_buff_cache",
        "swap_total",
        "swap_free",
        "swap_used",
        "mem_available",
    ];
    let mem_bytes_keys = &[
        "mem_total",
        "mem_free",
        "mem_used",
        "mem_buff_cache",
        "mem_available",
    ];
    let swap_bytes_keys = &["swap_total", "swap_free", "swap_used"];

    let mut out: Map<String, Value> = Map::new();

    // Extract raw string values for bytes computation
    let mem_unit = match raw.get("mem_unit") {
        Some(Value::String(s)) => s.clone(),
        _ => String::new(),
    };
    let swap_unit = match raw.get("swap_unit") {
        Some(Value::String(s)) => s.clone(),
        _ => String::new(),
    };

    // First pass: compute _bytes fields from raw strings, then convert types
    for (key, val) in &raw {
        let v = match val {
            Value::String(s) => {
                if root_int_list.contains(&key.as_str()) {
                    convert_to_int(s)
                        .map(|n| Value::Number(n.into()))
                        .unwrap_or(Value::Null)
                } else if root_float_list.contains(&key.as_str()) {
                    convert_to_float(s)
                        .and_then(serde_json::Number::from_f64)
                        .map(Value::Number)
                        .unwrap_or(Value::Null)
                } else {
                    val.clone()
                }
            }
            _ => val.clone(),
        };
        out.insert(key.clone(), v);
    }

    // Add _bytes fields for mem (using raw strings)
    for key in mem_bytes_keys.iter() {
        if let Some(Value::String(s)) = raw.get(*key) {
            let size_str = format!("{}{}", s, mem_unit);
            if let Some(bytes) = convert_size_to_int(&size_str, false) {
                out.insert(format!("{}_bytes", key), Value::Number(bytes.into()));
            }
        }
    }

    // Add _bytes fields for swap
    for key in swap_bytes_keys.iter() {
        if let Some(Value::String(s)) = raw.get(*key) {
            let size_str = format!("{}{}", s, swap_unit);
            if let Some(bytes) = convert_size_to_int(&size_str, false) {
                out.insert(format!("{}_bytes", key), Value::Number(bytes.into()));
            }
        }
    }

    out
}

fn parse_process_table(table_str: &str) -> Vec<Value> {
    let rows = sparse_table_parse(table_str);

    rows.into_iter()
        .map(|row| {
            // Step 1: Rename keys using key_map
            let mut renamed: Map<String, Value> = Map::new();
            for (old_key, val) in row {
                let new_key = process_key_map(&old_key)
                    .map(|s| s.to_string())
                    .unwrap_or(old_key);
                renamed.insert(new_key, val);
            }

            // Step 2: Process values
            let mut out: Map<String, Value> = Map::new();

            // Collect bytes info first (before type conversion)
            let mut extra_fields: Vec<(String, Value)> = Vec::new();

            for (key, val) in &renamed {
                let val_str = match val {
                    Value::String(s) => Some(s.as_str()),
                    _ => None,
                };

                // Set dashes to null
                let val = if val_str == Some("-") {
                    Value::Null
                } else {
                    val.clone()
                };

                // Cleanup trailing " -" from value
                let val = match &val {
                    Value::String(s) if s.ends_with(" -") => {
                        Value::String(s[..s.len() - 2].trim_end().to_string())
                    }
                    _ => val,
                };

                let val_str_clean = match &val {
                    Value::String(s) => Some(s.as_str()),
                    _ => None,
                };

                // Handle bytes fields
                if BYTES_KEYS.contains(&key.as_str())
                    && let Some(s) = val_str_clean
                {
                    let last_char = s.chars().last();
                    let unit = match last_char {
                        Some(c) if !c.is_ascii_digit() => c.to_string(),
                        _ => "b".to_string(),
                    };
                    if let Some(bytes) = convert_size_to_int(s, true) {
                        extra_fields.push((format!("{}_unit", key), Value::String(unit)));
                        extra_fields.push((format!("{}_bytes", key), Value::Number(bytes.into())));
                    }
                }

                // Type conversion
                let converted = if let Some(s) = val_str_clean {
                    if INT_KEYS.contains(&key.as_str()) {
                        convert_to_int(s)
                            .map(|n| Value::Number(n.into()))
                            .unwrap_or(Value::Null)
                    } else if FLOAT_KEYS.contains(&key.as_str()) {
                        convert_to_float(s)
                            .and_then(serde_json::Number::from_f64)
                            .map(Value::Number)
                            .unwrap_or(Value::Null)
                    } else {
                        val.clone()
                    }
                } else {
                    val.clone()
                };

                out.insert(key.clone(), converted);
            }

            // Apply status mapping
            if let Some(Value::String(s)) = out.get("status") {
                let mapped = map_status(s).to_string();
                out.insert("status".to_string(), Value::String(mapped));
            }

            // Split supplementary_gids
            if let Some(Value::String(s)) = out.get("supplementary_gids").cloned() {
                let ids: Vec<Value> = s
                    .split(',')
                    .filter(|p| !p.trim().is_empty() && !p.ends_with('+'))
                    .filter_map(|p| convert_to_int(p).map(|n| Value::Number(n.into())))
                    .collect();
                out.insert("supplementary_gids".to_string(), Value::Array(ids));
            }

            // Split supplementary_groups
            if let Some(Value::String(s)) = out.get("supplementary_groups").cloned() {
                let groups: Vec<Value> = s
                    .split(',')
                    .filter(|p| !p.trim().is_empty() && !p.ends_with('+'))
                    .map(|p| Value::String(p.to_string()))
                    .collect();
                out.insert("supplementary_groups".to_string(), Value::Array(groups));
            }

            // Split environment_variables
            if let Some(Value::String(s)) = out.get("environment_variables").cloned() {
                let vars: Vec<Value> = s
                    .split(' ')
                    .filter(|p| !p.trim().is_empty() && !p.ends_with('+'))
                    .map(|p| Value::String(p.to_string()))
                    .collect();
                out.insert("environment_variables".to_string(), Value::Array(vars));
            }

            // Add extra _unit and _bytes fields
            for (k, v) in extra_fields {
                out.insert(k, v);
            }

            Value::Object(out)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_top_gib_centos() {
        let input = include_str!("../../../../tests/fixtures/centos-7.7/top-b-n1-gib.out");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/centos-7.7/top-b-n1-gib.json"
        ))
        .unwrap();
        let parser = TopParser;
        let result = parser.parse(input, false).unwrap();
        let result_value: serde_json::Value = serde_json::to_value(result).unwrap();
        assert_eq!(result_value, expected);
    }

    #[test]
    fn test_top_n3_centos() {
        let input = include_str!("../../../../tests/fixtures/centos-7.7/top-b-n3.out");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/centos-7.7/top-b-n3.json"
        ))
        .unwrap();
        let parser = TopParser;
        let result = parser.parse(input, false).unwrap();
        let result_value: serde_json::Value = serde_json::to_value(result).unwrap();
        assert_eq!(result_value, expected);
    }

    #[test]
    fn test_top_ubuntu2010() {
        let input = include_str!("../../../../tests/fixtures/ubuntu-20.10/top-b-n1.out");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/ubuntu-20.10/top-b-n1.json"
        ))
        .unwrap();
        let parser = TopParser;
        let result = parser.parse(input, false).unwrap();
        let result_value: serde_json::Value = serde_json::to_value(result).unwrap();
        assert_eq!(result_value, expected);
    }
}
