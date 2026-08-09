//! Streaming parser for `iostat` command output.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::{LineParser, Parser, Record, StreamingParser, parse_via_session};
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};

use super::iostat::{create_obj_list, normalize_iostat_header};

pub struct IostatSParser;

static INFO: ParserInfo = ParserInfo {
    name: "iostat_s",
    argument: "--iostat-s",
    version: "1.1.0",
    description: "Streaming `iostat` command parser",
    author: "jc-rs contributors",
    author_email: "os@g1sw.com",
    compatible: &[Platform::Linux],
    tags: &[Tag::Command, Tag::Streaming],
    magic_commands: &[],
    streaming: true,
    hidden: false,
    deprecated: false,
};

static IOSTAT_S_PARSER: IostatSParser = IostatSParser;

inventory::submit! {
    ParserEntry::new(&IOSTAT_S_PARSER)
}

impl Parser for IostatSParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, quiet: bool) -> Result<ParseOutput, ParseError> {
        parse_via_session(self, input, quiet)
    }

    fn as_streaming(&self) -> Option<&dyn StreamingParser> {
        Some(self)
    }
}

impl StreamingParser for IostatSParser {
    fn session(&self) -> Box<dyn LineParser> {
        Box::new(IostatSession::default())
    }
}

/// `iostat` alternates between an `avg-cpu:` block and a `Device` block, each
/// introduced by its own header. The session remembers which block it is in and
/// what that block's header was, which is all a data line needs to be turned
/// into a record on arrival.
#[derive(Default)]
struct IostatSession {
    section: Option<Section>,
    header: String,
}

#[derive(PartialEq, Clone, Copy)]
enum Section {
    Cpu,
    Device,
}

impl Section {
    fn name(self) -> &'static str {
        match self {
            Section::Cpu => "cpu",
            Section::Device => "device",
        }
    }
}

impl LineParser for IostatSession {
    fn parse_line(&mut self, line: &str, _quiet: bool) -> Result<Option<Record>, ParseError> {
        if line.trim().is_empty() {
            return Ok(None);
        }

        if line.starts_with("avg-cpu:") {
            self.section = Some(Section::Cpu);
            self.header = normalize_iostat_header(&line[8..]).trim().to_string();
            return Ok(None);
        }

        if line.starts_with("Device") {
            self.section = Some(Section::Device);
            self.header = normalize_iostat_header(line).replace(':', " ");
            return Ok(None);
        }

        // Anything before the first header is the `Linux 3.10.0-...` banner.
        let Some(section) = self.section else {
            return Ok(None);
        };

        let table = format!("{}\n{}", self.header, line);
        Ok(create_obj_list(&table, section.name()).into_iter().next())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_iostat_s_centos() {
        let input = include_str!("../../../../tests/fixtures/centos-7.7/iostat.out");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/centos-7.7/iostat-streaming.json"
        ))
        .unwrap();
        let parser = IostatSParser;
        let result = parser.parse(input, false).unwrap();
        let result_value: serde_json::Value = serde_json::to_value(result).unwrap();
        assert_eq!(result_value, expected);
    }

    #[test]
    fn test_iostat_s_ubuntu18() {
        let input = include_str!("../../../../tests/fixtures/ubuntu-18.04/iostat.out");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/ubuntu-18.04/iostat-streaming.json"
        ))
        .unwrap();
        let parser = IostatSParser;
        let result = parser.parse(input, false).unwrap();
        let result_value: serde_json::Value = serde_json::to_value(result).unwrap();
        assert_eq!(result_value, expected);
    }
}
