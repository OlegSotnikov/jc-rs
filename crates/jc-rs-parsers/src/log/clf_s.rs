//! Common Log Format (CLF) streaming parser.

use super::clf::parse_clf_line;
use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::{
    FnSession, LineParser, Parser, Record, StreamingParser, parse_via_session,
};
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};

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
        if input.trim().is_empty() {
            return Err(ParseError::InvalidInput("empty input".to_string()));
        }
        parse_via_session(self, input, quiet)
    }

    fn as_streaming(&self) -> Option<&dyn StreamingParser> {
        Some(self)
    }
}

impl StreamingParser for ClfSParser {
    fn session(&self) -> Box<dyn LineParser> {
        Box::new(FnSession::new(clf_line))
    }
}

/// Every CLF line stands alone, so the session carries no state.
fn clf_line(line: &str, _quiet: bool) -> Result<Option<Record>, ParseError> {
    if line.trim().is_empty() {
        return Ok(None);
    }
    Ok(Some(parse_clf_line(line)))
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
        let mut session = ClfSParser.session();
        let line = r#"127.0.0.1 user-identifier frank [10/Oct/2000:13:55:36 -0700] "GET /apache_pb.gif HTTP/1.0" 200 2326"#;
        let record = session.parse_line(line, false).unwrap().unwrap();
        let v = serde_json::Value::Object(record);
        assert_eq!(v["host"], "127.0.0.1");
        assert_eq!(v["status"], 200);
    }

    #[test]
    fn test_clf_s_skip_empty() {
        let mut session = ClfSParser.session();
        assert!(session.parse_line("", false).unwrap().is_none());
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
