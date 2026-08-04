//! Parser for `uptime` command output.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use regex::Regex;
use serde_json::{Map, Value};
use std::sync::OnceLock;

pub struct UptimeParser;

static INFO: ParserInfo = ParserInfo {
    name: "uptime",
    argument: "--uptime",
    version: "1.5.0",
    description: "Converts `uptime` command output to JSON",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[Platform::Linux, Platform::Darwin, Platform::FreeBSD],
    tags: &[Tag::Command],
    magic_commands: &["uptime"],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static UPTIME_PARSER: UptimeParser = UptimeParser;

inventory::submit! {
    ParserEntry::new(&UPTIME_PARSER)
}

impl Parser for UptimeParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        let record = parse_uptime(input.trim())?;
        Ok(ParseOutput::Object(record))
    }
}

fn uptime_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"(?x)
            ^\s*
            (?P<time>\d{1,2}:\d{2}(?::\d{2})?)   # current time HH:MM or HH:MM:SS
            \s+up\s+
            (?P<uptime>.+?)                         # uptime string (greedy-lazy)
            ,\s*
            (?P<users>\d+)\s+users?\s*,\s*
            load\s+averages?:\s*
            (?P<load1>[\d.]+)[,\s]+
            (?P<load5>[\d.]+)[,\s]+
            (?P<load15>[\d.]+)
        ",
        )
        .unwrap()
    })
}

fn parse_uptime(input: &str) -> Result<Map<String, Value>, ParseError> {
    let re = uptime_re();
    let caps = re
        .captures(input)
        .ok_or_else(|| ParseError::InvalidInput(format!("Cannot parse uptime: {}", input)))?;

    let time_str = caps["time"].to_string();
    let uptime_str = caps["uptime"].trim().to_string();
    let users: i64 = caps["users"].parse().unwrap_or(0);
    let load1: f64 = caps["load1"].parse().unwrap_or(0.0);
    let load5: f64 = caps["load5"].parse().unwrap_or(0.0);
    let load15: f64 = caps["load15"].parse().unwrap_or(0.0);

    // Parse time
    let time_parts: Vec<&str> = time_str.split(':').collect();
    let time_hour: i64 = time_parts.first().and_then(|s| s.parse().ok()).unwrap_or(0);
    let time_minute: i64 = time_parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);
    let time_second: Option<i64> = time_parts.get(2).and_then(|s| s.parse().ok());

    // Parse uptime string into days, hours, minutes
    let (uptime_days, uptime_hours, uptime_minutes) = parse_uptime_duration(&uptime_str);
    let uptime_total_seconds: i64 = uptime_days * 86400 + uptime_hours * 3600 + uptime_minutes * 60;

    let mut record = Map::new();
    record.insert("time".to_string(), Value::String(time_str));
    record.insert("uptime".to_string(), Value::String(uptime_str));
    record.insert("users".to_string(), Value::Number(users.into()));
    record.insert(
        "load_1m".to_string(),
        serde_json::Number::from_f64(load1)
            .map(Value::Number)
            .unwrap_or(Value::Null),
    );
    record.insert(
        "load_5m".to_string(),
        serde_json::Number::from_f64(load5)
            .map(Value::Number)
            .unwrap_or(Value::Null),
    );
    record.insert(
        "load_15m".to_string(),
        serde_json::Number::from_f64(load15)
            .map(Value::Number)
            .unwrap_or(Value::Null),
    );
    record.insert("time_hour".to_string(), Value::Number(time_hour.into()));
    record.insert("time_minute".to_string(), Value::Number(time_minute.into()));
    record.insert(
        "time_second".to_string(),
        time_second
            .map(|s| Value::Number(s.into()))
            .unwrap_or(Value::Null),
    );
    record.insert("uptime_days".to_string(), Value::Number(uptime_days.into()));
    record.insert(
        "uptime_hours".to_string(),
        Value::Number(uptime_hours.into()),
    );
    record.insert(
        "uptime_minutes".to_string(),
        Value::Number(uptime_minutes.into()),
    );
    record.insert(
        "uptime_total_seconds".to_string(),
        Value::Number(uptime_total_seconds.into()),
    );

    Ok(record)
}

fn days_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(\d+)\s+days?").unwrap())
}

fn mins_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(\d+)\s+min").unwrap())
}

fn hm_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(\d+):(\d+)\s*$").unwrap())
}

/// Parse uptime duration string like:
/// - "16:03" → (0, 16, 3)
/// - "2 days, 19:32" → (2, 19, 32)
/// - "3 days, 4:03" → (3, 4, 3)
/// - "10 min" → (0, 0, 10)
/// - "1 day, 2:03" → (1, 2, 3)
fn parse_uptime_duration(s: &str) -> (i64, i64, i64) {
    let mut days: i64 = 0;
    let mut hours: i64 = 0;
    let mut minutes: i64 = 0;

    // Extract days
    if let Some(caps) = days_re().captures(s) {
        days = caps[1].parse().unwrap_or(0);
    }

    // Check for "X min" pattern (no hours)
    if let Some(caps) = mins_re().captures(s) {
        minutes = caps[1].parse().unwrap_or(0);
        return (days, hours, minutes);
    }

    // Look for H:MM pattern
    if let Some(caps) = hm_re().captures(s) {
        hours = caps[1].parse().unwrap_or(0);
        minutes = caps[2].parse().unwrap_or(0);
    }

    (days, hours, minutes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uptime_centos() {
        let input = include_str!("../../../../tests/fixtures/centos-7.7/uptime.out");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/centos-7.7/uptime.json"
        ))
        .unwrap();

        let parser = UptimeParser;
        let result = parser.parse(input, false).unwrap();
        let result_value: serde_json::Value = serde_json::to_value(result).unwrap();
        assert_eq!(result_value, expected);
    }

    #[test]
    fn test_uptime_ubuntu18() {
        let input = include_str!("../../../../tests/fixtures/ubuntu-18.04/uptime.out");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/ubuntu-18.04/uptime.json"
        ))
        .unwrap();

        let parser = UptimeParser;
        let result = parser.parse(input, false).unwrap();
        let result_value: serde_json::Value = serde_json::to_value(result).unwrap();
        assert_eq!(result_value, expected);
    }

    #[test]
    fn test_uptime_osx() {
        let input = include_str!("../../../../tests/fixtures/osx-10.14.6/uptime.out");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/osx-10.14.6/uptime.json"
        ))
        .unwrap();

        let parser = UptimeParser;
        let result = parser.parse(input, false).unwrap();
        let result_value: serde_json::Value = serde_json::to_value(result).unwrap();
        assert_eq!(result_value, expected);
    }
}
