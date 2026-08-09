//! Dummy parser: proves the inventory registration system works end to end.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};

struct DummyParser;

static DUMMY_INFO: ParserInfo = ParserInfo {
    name: "dummy",
    argument: "--dummy",
    version: "1.0.0",
    description: "Dummy parser for testing registration",
    author: "jc-rs contributors",
    author_email: "os@g1sw.com",
    compatible: &[Platform::Universal],
    tags: &[Tag::String],
    magic_commands: &[],
    streaming: false,
    hidden: true,
    deprecated: false,
};

impl Parser for DummyParser {
    fn info(&self) -> &'static ParserInfo {
        &DUMMY_INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        let mut map = serde_json::Map::new();
        map.insert(
            "input".to_string(),
            serde_json::Value::String(input.to_string()),
        );
        Ok(ParseOutput::Object(map))
    }
}

static DUMMY_PARSER_INSTANCE: DummyParser = DummyParser;

inventory::submit! {
    ParserEntry::new(&DUMMY_PARSER_INSTANCE)
}
