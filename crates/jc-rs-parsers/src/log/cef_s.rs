//! Common Event Format (CEF) streaming parser.

use super::cef::parse_cef_line;
use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::{Parser, StreamingParser};
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};

struct CefSParser;

static INFO: ParserInfo = ParserInfo {
    name: "cef_s",
    argument: "--cef-s",
    version: "1.0.0",
    description: "Common Event Format (CEF) string streaming parser",
    author: "jc-rs contributors",
    author_email: "jc-rs@example.com",
    compatible: &[Platform::Universal],
    tags: &[Tag::String, Tag::File, Tag::Streaming],
    magic_commands: &[],
    streaming: true,
    hidden: false,
    deprecated: false,
};

impl Parser for CefSParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, quiet: bool) -> Result<ParseOutput, ParseError> {
        let mut records = Vec::new();
        for line in input.lines() {
            if line.trim().is_empty() {
                continue;
            }
            match self.parse_line(line, quiet)? {
                Some(ParseOutput::Object(map)) => records.push(map),
                _ => {}
            }
        }
        Ok(ParseOutput::Array(records))
    }
}

impl StreamingParser for CefSParser {
    fn parse_line(&self, line: &str, _quiet: bool) -> Result<Option<ParseOutput>, ParseError> {
        if line.trim().is_empty() {
            return Ok(None);
        }
        Ok(Some(ParseOutput::Object(parse_cef_line(line))))
    }
}

static INSTANCE: CefSParser = CefSParser;

inventory::submit! {
    ParserEntry::new(&INSTANCE)
}

#[cfg(test)]
mod tests {
    use super::*;
    use jc_rs_core::registry::find_parser;

    #[test]
    fn test_cef_s_registered() {
        assert!(find_parser("cef_s").is_some());
    }
}
