//! Parser for `/proc/<pid>/stat`

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};

pub struct ProcPidStatParser;

static INFO: ParserInfo = ParserInfo {
    name: "proc_pid_stat",
    argument: "--proc-pid-stat",
    version: "1.0.0",
    description: "Converts `/proc/<pid>/stat` file to JSON",
    author: "jc-rs contributors",
    author_email: "os@g1sw.com",
    compatible: &[Platform::Linux],
    tags: &[Tag::File, Tag::Slurpable],
    magic_commands: &[],
    streaming: false,
    hidden: true,
    deprecated: false,
};

static PROC_PID_STAT_PARSER: ProcPidStatParser = ProcPidStatParser;

inventory::submit! { ParserEntry::new(&PROC_PID_STAT_PARSER) }

fn state_pretty(state: &str) -> &'static str {
    match state {
        "R" => "Running",
        "S" => "Sleeping in an interruptible wait",
        "D" => "Waiting in uninterruptible disk sleep",
        "Z" => "Zombie",
        "T" => "Stopped (on a signal) or trace stopped",
        "t" => "Tracing stop",
        "W" => "Waking",
        "X" => "Dead",
        "x" => "Dead",
        "K" => "Wakekill",
        "P" => "Parked",
        "I" => "Idle",
        _ => "Unknown",
    }
}

impl Parser for ProcPidStatParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Err(ParseError::Generic("Empty input".to_string()));
        }

        // Field names after state (index 2 onwards)
        let field_names = [
            "pid",                   // 0 - before comm
            "comm",                  // 1 - in parens
            "state",                 // 2
            "ppid",                  // 3
            "pgrp",                  // 4
            "session",               // 5
            "tty_nr",                // 6
            "tpg_id",                // 7
            "flags",                 // 8
            "minflt",                // 9
            "cminflt",               // 10
            "majflt",                // 11
            "cmajflt",               // 12
            "utime",                 // 13
            "stime",                 // 14
            "cutime",                // 15
            "cstime",                // 16
            "priority",              // 17
            "nice",                  // 18
            "num_threads",           // 19
            "itrealvalue",           // 20
            "starttime",             // 21
            "vsize",                 // 22
            "rss",                   // 23
            "rsslim",                // 24
            "startcode",             // 25
            "endcode",               // 26
            "startstack",            // 27
            "kstkeep",               // 28
            "kstkeip",               // 29
            "signal",                // 30
            "blocked",               // 31
            "sigignore",             // 32
            "sigcatch",              // 33
            "wchan",                 // 34
            "nswap",                 // 35
            "cnswap",                // 36
            "exit_signal",           // 37
            "processor",             // 38
            "rt_priority",           // 39
            "policy",                // 40
            "delayacct_blkio_ticks", // 41
            "guest_time",            // 42
            "cguest_time",           // 43
            "start_data",            // 44
            "end_data",              // 45
            "start_brk",             // 46
            "arg_start",             // 47
            "arg_end",               // 48
            "env_start",             // 49
            "env_end",               // 50
            "exit_code",             // 51
        ];

        let mut map: Map<String, Value> = Map::new();

        // Find the first '(' and the LAST ')'
        let open_paren = input
            .find('(')
            .ok_or_else(|| ParseError::Generic("Missing '(' in stat".to_string()))?;
        let close_paren = input
            .rfind(')')
            .ok_or_else(|| ParseError::Generic("Missing ')' in stat".to_string()))?;

        // pid is before '('
        let pid_str = input[..open_paren].trim();
        let pid: u64 = pid_str
            .parse()
            .map_err(|_| ParseError::Generic("Invalid pid".to_string()))?;
        map.insert("pid".to_string(), Value::Number(pid.into()));

        // comm is between parens
        let comm = &input[open_paren + 1..close_paren];
        map.insert("comm".to_string(), Value::String(comm.to_string()));

        // Remaining fields after ')'
        let rest = input[close_paren + 1..].trim();
        let parts: Vec<&str> = rest.split_whitespace().collect();

        // parts[0] = state, parts[1..] = numeric fields
        // field_names[2] = "state", field_names[3..] = numeric fields
        for (i, part) in parts.iter().enumerate() {
            let field_idx = i + 2; // offset into field_names
            if field_idx >= field_names.len() {
                break;
            }
            let name: &str = field_names[field_idx];
            if name == "state" {
                let s: &str = part;
                map.insert("state".to_string(), Value::String(s.to_string()));
                map.insert(
                    "state_pretty".to_string(),
                    Value::String(state_pretty(s).to_string()),
                );
            } else {
                // Parse as u64 first (for large values like rsslim), fall back to i64
                if let Ok(v) = part.parse::<u64>() {
                    let n: serde_json::Number = v.into();
                    map.insert(name.to_string(), Value::Number(n));
                } else if let Ok(v) = part.parse::<i64>() {
                    let n: serde_json::Number = v.into();
                    map.insert(name.to_string(), Value::Number(n));
                }
            }
        }

        Ok(ParseOutput::Object(map))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proc_pid_stat() {
        let input = include_str!("../../../../tests/fixtures/linux-proc/pid_stat");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/linux-proc/pid_stat.json"
        ))
        .unwrap();
        let parser = ProcPidStatParser;
        let result = parser.parse(input, false).unwrap();
        let got = serde_json::to_value(result).unwrap();
        assert_eq!(got, expected);
    }

    #[test]
    fn test_proc_pid_stat_hack() {
        let input = include_str!("../../../../tests/fixtures/linux-proc/pid_stat_hack");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/linux-proc/pid_stat_hack.json"
        ))
        .unwrap();
        let parser = ProcPidStatParser;
        let result = parser.parse(input, false).unwrap();
        let got = serde_json::to_value(result).unwrap();
        assert_eq!(got, expected);
    }
}
