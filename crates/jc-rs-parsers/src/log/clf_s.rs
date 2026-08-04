//! Common Log Format (CLF) streaming parser.

use super::clf::parse_clf_line;
use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::{Parser, StreamingParser};
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::Map;

struct ClfSParser;

static CLF_S_INFO: ParserInfo = ParserInfo {
    name: "clf_s",
    argument: "--clf-s",
    version: "1.0.0",
    description: "Common and Combined Log Format file streaming parser",
    author: "jc-rs contributors",
    author_email: "jc-rs@example.com",
    compatible: &[Platform::Universal],
    tags: &[Tag::Command, Tag::Slurpable, Tag::Streaming],
    magic_commands: &[],
    streaming: true,
    hidden: false,
    deprecated: false,
};

impl Parser for ClfSParser {
    fn info(&self) -> &'static ParserInfo {
        &CLF_S_INFO
    }

    fn parse(&self, input: &str, quiet: bool) -> Result<ParseOutput, ParseError> {
        let input = input.trim();
        if input.is_empty() {
            return Err(ParseError::InvalidInput("empty input".to_string()));
        }

        let mut records: Vec<Map<String, serde_json::Value>> = Vec::new();
        for line in input.lines() {
            if line.trim().is_empty() {
                continue;
            }
            match self.parse_line(line, quiet)? {
                Some(ParseOutput::Object(map)) => records.push(map),
                Some(ParseOutput::Array(_)) => {}
                None => {}
            }
        }

        Ok(ParseOutput::Array(records))
    }
}

impl StreamingParser for ClfSParser {
    fn parse_line(&self, line: &str, _quiet: bool) -> Result<Option<ParseOutput>, ParseError> {
        if line.trim().is_empty() {
            return Ok(None);
        }
        let map = parse_clf_line(line);
        Ok(Some(ParseOutput::Object(map)))
    }
}

static CLF_S_PARSER_INSTANCE: ClfSParser = ClfSParser;

inventory::submit! {
    ParserEntry::new(&CLF_S_PARSER_INSTANCE)
}

#[cfg(test)]
mod tests {
    use super::*;
    use jc_rs_core::types::ParseOutput;

    #[test]
    fn test_clf_s_parse_line() {
        let parser = ClfSParser;
        let line = r#"127.0.0.1 user-identifier frank [10/Oct/2000:13:55:36 -0700] "GET /apache_pb.gif HTTP/1.0" 200 2326"#;
        let result = parser.parse_line(line, false).unwrap();
        assert!(result.is_some());
        match result.unwrap() {
            ParseOutput::Object(map) => {
                let v = serde_json::Value::Object(map);
                assert_eq!(v["host"], "127.0.0.1");
                assert_eq!(v["status"], 200);
            }
            _ => panic!("expected object"),
        }
    }

    #[test]
    fn test_clf_s_skip_empty() {
        let parser = ClfSParser;
        let result = parser.parse_line("", false).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_clf_s_full_parse() {
        let parser = ClfSParser;
        let input = concat!(
            "127.0.0.1 - frank [10/Oct/2000:13:55:36 -0700] \"GET /index HTTP/1.0\" 200 512\n",
            "1.2.3.4 - - [11/Nov/2016:03:04:55 +0100] \"POST /api HTTP/1.1\" 201 128\n"
        );
        let result = parser.parse(input, false).unwrap();
        match result {
            ParseOutput::Array(arr) => {
                assert_eq!(arr.len(), 2);
            }
            _ => panic!("expected array"),
        }
    }
}
