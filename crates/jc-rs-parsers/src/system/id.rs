//! Parser for `id` command output.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use regex::Regex;
use serde_json::{Map, Value};
use std::sync::OnceLock;

pub struct IdParser;

static INFO: ParserInfo = ParserInfo {
    name: "id",
    argument: "--id",
    version: "1.4.0",
    description: "Converts `id` command output to JSON",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[Platform::Linux, Platform::Darwin, Platform::FreeBSD],
    tags: &[Tag::Command],
    magic_commands: &["id"],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static ID_PARSER: IdParser = IdParser;

inventory::submit! {
    ParserEntry::new(&ID_PARSER)
}

impl Parser for IdParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        let record = parse_id(input.trim())?;
        Ok(ParseOutput::Object(record))
    }
}

fn id_entry_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(\d+)\(([^)]+)\)").unwrap())
}

fn context_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"context=([^:]+):([^:]+):([^:]+):(.+)$").unwrap())
}

fn parse_id_entry(s: &str) -> Option<Map<String, Value>> {
    let re = id_entry_re();
    let caps = re.captures(s)?;
    let id: i64 = caps[1].parse().ok()?;
    let name = caps[2].to_string();
    let mut m = Map::new();
    m.insert("id".to_string(), Value::Number(id.into()));
    m.insert("name".to_string(), Value::String(name));
    Some(m)
}

fn parse_id(input: &str) -> Result<Map<String, Value>, ParseError> {
    if input.is_empty() {
        return Err(ParseError::InvalidInput("id input is empty".to_string()));
    }

    let mut record = Map::new();

    // Parse uid=N(name)
    if let Some(uid_start) = input.find("uid=") {
        let uid_str = &input[uid_start + 4..];
        let uid_end = uid_str.find(' ').unwrap_or(uid_str.len());
        if let Some(uid_map) = parse_id_entry(&uid_str[..uid_end]) {
            record.insert("uid".to_string(), Value::Object(uid_map));
        }
    }

    // Parse gid=N(name)
    if let Some(gid_start) = input.find("gid=") {
        let gid_str = &input[gid_start + 4..];
        let gid_end = gid_str.find(' ').unwrap_or(gid_str.len());
        if let Some(gid_map) = parse_id_entry(&gid_str[..gid_end]) {
            record.insert("gid".to_string(), Value::Object(gid_map));
        }
    }

    // Parse groups=N(name),N(name),...
    if let Some(groups_start) = input.find("groups=") {
        let groups_str = &input[groups_start + 7..];
        // Groups end at space or end of groups section (before context if present)
        let groups_end = groups_str.find(" context=").unwrap_or(groups_str.len());
        let groups_part = &groups_str[..groups_end];

        let re = id_entry_re();
        let groups: Vec<Value> = re
            .captures_iter(groups_part)
            .filter_map(|caps| {
                let id: i64 = caps[1].parse().ok()?;
                let name = caps[2].to_string();
                let mut m = Map::new();
                m.insert("id".to_string(), Value::Number(id.into()));
                m.insert("name".to_string(), Value::String(name));
                Some(Value::Object(m))
            })
            .collect();
        record.insert("groups".to_string(), Value::Array(groups));
    }

    // Parse context=user:role:type:level (SELinux)
    {
        let ctx_re = context_re();
        if let Some(caps) = ctx_re.captures(input) {
            let mut ctx = Map::new();
            ctx.insert("user".to_string(), Value::String(caps[1].to_string()));
            ctx.insert("role".to_string(), Value::String(caps[2].to_string()));
            ctx.insert("type".to_string(), Value::String(caps[3].to_string()));
            ctx.insert("level".to_string(), Value::String(caps[4].to_string()));
            record.insert("context".to_string(), Value::Object(ctx));
        }
    }

    Ok(record)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_id_centos() {
        let input = include_str!("../../../../tests/fixtures/centos-7.7/id.out");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/centos-7.7/id.json"
        ))
        .unwrap();

        let parser = IdParser;
        let result = parser.parse(input, false).unwrap();
        let result_value: serde_json::Value = serde_json::to_value(result).unwrap();
        assert_eq!(result_value, expected);
    }

    #[test]
    fn test_id_osx() {
        let input = include_str!("../../../../tests/fixtures/osx-10.14.6/id.out");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/osx-10.14.6/id.json"
        ))
        .unwrap();

        let parser = IdParser;
        let result = parser.parse(input, false).unwrap();
        let result_value: serde_json::Value = serde_json::to_value(result).unwrap();
        assert_eq!(result_value, expected);
    }
}
