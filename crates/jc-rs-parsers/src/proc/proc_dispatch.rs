//! Meta-parser for `/proc` files — dispatches to specific parsers by content detection.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::{ParserEntry, find_parser};
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use regex::Regex;

pub struct ProcDispatchParser;

static INFO: ParserInfo = ParserInfo {
    name: "proc",
    argument: "--proc",
    version: "1.0.0",
    description: "Automatically identifies and parses `/proc` files",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[Platform::Linux],
    tags: &[Tag::File, Tag::Slurpable],
    magic_commands: &[],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static PROC_DISPATCH_PARSER: ProcDispatchParser = ProcDispatchParser;

inventory::submit! { ParserEntry::new(&PROC_DISPATCH_PARSER) }

impl Parser for ProcDispatchParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Err(ParseError::InvalidInput(
                "Proc file could not be identified.".to_string(),
            ));
        }

        // Signature patterns and their corresponding parser names, ordered to avoid ambiguity
        let signatures: Vec<(&str, &str)> = vec![
            (r"^processor\t+: \d+", "proc_cpuinfo"),
            (r"^MemTotal:.*\nMemFree:", "proc_meminfo"),
            (r"^cpu\s+(?: \d+){7,10}", "proc_stat"),
            (
                r"^\d+\.\d\d \d+\.\d\d \d+\.\d\d \d+/\d+ \d+$",
                "proc_loadavg",
            ),
            (r"^\d+\.\d\d \d+\.\d\d$", "proc_uptime"),
            (r"^.+\sversion\s[^\n]+$", "proc_version"),
            (r"^BOOT_IMAGE=", "proc_cmdline"),
            (
                r"^\w+\s+[\-WUR]{3} \([ECBpba ]+\)\s+\d+:\d+",
                "proc_consoles",
            ),
            (r"^name\s+:.*\ndriver\s+:.*\nmodule\s+:", "proc_crypto"),
            (r"^Character devices:\n\s+\d+ ", "proc_devices"),
            (r"^(?:(?:nodev\t|\t)\w+\n){3}", "proc_filesystems"),
        ];

        for (pattern, parser_name) in &signatures {
            let re = match Regex::new(pattern) {
                Ok(r) => r,
                Err(_) => continue,
            };
            if re.is_match(input) {
                if let Some(parser) = find_parser(parser_name) {
                    return parser.parse(input, quiet);
                }
            }
        }

        Err(ParseError::InvalidInput(
            "Proc file could not be identified.".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proc_dispatch_meminfo() {
        let input = include_str!("../../../../tests/fixtures/linux-proc/meminfo");
        let parser = ProcDispatchParser;
        let result = parser.parse(input, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_proc_dispatch_cpuinfo() {
        let input = include_str!("../../../../tests/fixtures/linux-proc/cpuinfo");
        let parser = ProcDispatchParser;
        let result = parser.parse(input, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_proc_dispatch_empty() {
        let parser = ProcDispatchParser;
        let result = parser.parse("", false);
        assert!(result.is_err());
    }
}
