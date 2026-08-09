//! Parser for `/proc/cmdline`

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};

pub struct ProcCmdlineParser;

static INFO: ParserInfo = ParserInfo {
    name: "proc_cmdline",
    argument: "--proc-cmdline",
    version: "1.0.0",
    description: "Converts `/proc/cmdline` file to JSON",
    author: "jc-rs contributors",
    author_email: "os@g1sw.com",
    compatible: &[Platform::Linux],
    tags: &[Tag::File, Tag::Slurpable],
    magic_commands: &["/proc/cmdline"],
    streaming: false,
    hidden: true,
    deprecated: false,
};

static PROC_CMDLINE_PARSER: ProcCmdlineParser = ProcCmdlineParser;

inventory::submit! { ParserEntry::new(&PROC_CMDLINE_PARSER) }

impl Parser for ProcCmdlineParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        let line = input.trim();
        if line.is_empty() {
            return Ok(ParseOutput::Object(Map::new()));
        }

        let mut out = Map::new();

        for token in line.split_whitespace() {
            if let Some(eq_pos) = token.find('=') {
                let key = &token[..eq_pos];
                let val = &token[eq_pos + 1..];
                out.insert(key.to_string(), Value::String(val.to_string()));
            } else {
                out.insert(token.to_string(), Value::Bool(true));
            }
        }

        Ok(ParseOutput::Object(out))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proc_cmdline_basic() {
        let input = "BOOT_IMAGE=/vmlinuz root=/dev/sda1 ro quiet";
        let parser = ProcCmdlineParser;
        let result = parser.parse(input, false).unwrap();
        let got = serde_json::to_value(result).unwrap();
        assert_eq!(got["BOOT_IMAGE"], "/vmlinuz");
        assert_eq!(got["root"], "/dev/sda1");
        assert_eq!(got["ro"], true);
        assert_eq!(got["quiet"], true);
    }
}
