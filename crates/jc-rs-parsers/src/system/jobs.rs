//! Parser for `jobs` shell builtin output.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};

pub struct JobsParser;

static INFO: ParserInfo = ParserInfo {
    name: "jobs",
    argument: "--jobs",
    version: "1.6.0",
    description: "Converts `jobs` command output to JSON",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[Platform::Linux, Platform::Darwin, Platform::FreeBSD],
    tags: &[Tag::Command],
    magic_commands: &[],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static JOBS_PARSER: JobsParser = JobsParser;

inventory::submit! {
    ParserEntry::new(&JOBS_PARSER)
}

impl Parser for JobsParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Array(Vec::new()));
        }

        let mut result = Vec::new();

        for line in input.lines().filter(|l| !l.trim().is_empty()) {
            let mut out = Map::new();
            let parts: Vec<&str> = line.split_whitespace().collect();

            if parts.is_empty() {
                continue;
            }

            // Check if -l was used (second field is a PID - starts with digit)
            let (job_field_idx, pid_opt, status_idx, cmd_start_idx) = if parts.len() >= 2
                && parts[1]
                    .chars()
                    .next()
                    .map(|c| c.is_ascii_digit())
                    .unwrap_or(false)
            {
                // -l format: [N]? pid status command
                (0, Some(parts[1]), 2, 3)
            } else {
                // no -l: [N]? status command
                (0, None, 1, 2)
            };

            let mut job_field = parts[job_field_idx].to_string();
            let mut history = None;

            // Check for + or -
            if job_field.contains('+') {
                history = Some("current");
                job_field = job_field.replace('+', "");
            } else if job_field.contains('-') {
                history = Some("previous");
                job_field = job_field.replace('-', "");
            }

            // Clean [N] -> N
            job_field = job_field
                .trim_start_matches('[')
                .trim_end_matches(']')
                .to_string();

            if let Ok(n) = job_field.parse::<i64>() {
                out.insert("job_number".to_string(), Value::Number(n.into()));
            } else {
                out.insert("job_number".to_string(), Value::String(job_field));
            }

            if let Some(pid_str) = pid_opt {
                if let Ok(n) = pid_str.parse::<i64>() {
                    out.insert("pid".to_string(), Value::Number(n.into()));
                } else {
                    out.insert("pid".to_string(), Value::String(pid_str.to_string()));
                }
            }

            if let Some(h) = history {
                out.insert("history".to_string(), Value::String(h.to_string()));
            }

            if let Some(&status) = parts.get(status_idx) {
                out.insert("status".to_string(), Value::String(status.to_string()));
            }

            if cmd_start_idx < parts.len() {
                let cmd = parts[cmd_start_idx..].join(" ");
                out.insert("command".to_string(), Value::String(cmd));
            }

            result.push(out);
        }

        Ok(ParseOutput::Array(result))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jobs_basic() {
        let input = "[1]   Running                 sleep 11 &\n[2]   Running                 sleep 12 &\n[3]-  Running                 sleep 13 &\n[4]+  Running                 sleep 14 &\n";
        let parser = JobsParser;
        let result = parser.parse(input, false).unwrap();
        if let ParseOutput::Array(arr) = result {
            assert_eq!(arr.len(), 4);
            assert_eq!(arr[0].get("job_number"), Some(&Value::Number(1.into())));
            assert_eq!(
                arr[0].get("status"),
                Some(&Value::String("Running".to_string()))
            );
            assert_eq!(
                arr[2].get("history"),
                Some(&Value::String("previous".to_string()))
            );
            assert_eq!(
                arr[3].get("history"),
                Some(&Value::String("current".to_string()))
            );
        } else {
            panic!("Expected Array");
        }
    }

    #[test]
    fn test_jobs_empty() {
        let parser = JobsParser;
        let result = parser.parse("", false).unwrap();
        if let ParseOutput::Array(arr) = result {
            assert!(arr.is_empty());
        } else {
            panic!("Expected Array");
        }
    }
}
