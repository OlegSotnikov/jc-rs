//! Unix Epoch Timestamp string parser.

use chrono::{DateTime, Datelike, Local, TimeZone, Timelike, Utc, Weekday};
use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};

struct TimestampParser;

static TIMESTAMP_INFO: ParserInfo = ParserInfo {
    name: "timestamp",
    argument: "--timestamp",
    version: "1.0.0",
    description: "Unix Epoch Timestamp string parser",
    author: "jc-rs contributors",
    author_email: "os@g1sw.com",
    compatible: &[Platform::Universal],
    tags: &[Tag::String],
    magic_commands: &[],
    streaming: false,
    hidden: false,
    deprecated: false,
};

fn weekday_abbr(w: Weekday) -> &'static str {
    match w {
        Weekday::Mon => "Mon",
        Weekday::Tue => "Tue",
        Weekday::Wed => "Wed",
        Weekday::Thu => "Thu",
        Weekday::Fri => "Fri",
        Weekday::Sat => "Sat",
        Weekday::Sun => "Sun",
    }
}

fn month_abbr(m: u32) -> &'static str {
    match m {
        1 => "Jan",
        2 => "Feb",
        3 => "Mar",
        4 => "Apr",
        5 => "May",
        6 => "Jun",
        7 => "Jul",
        8 => "Aug",
        9 => "Sep",
        10 => "Oct",
        11 => "Nov",
        12 => "Dec",
        _ => "???",
    }
}

fn datetime_to_map_naive(dt: &chrono::NaiveDateTime) -> Map<String, Value> {
    let hour_12 = {
        let h = dt.hour() % 12;
        if h == 0 { 12 } else { h }
    };
    let period = if dt.hour() < 12 { "AM" } else { "PM" };
    let weekday_num = dt.weekday().number_from_monday();
    let day_of_year = dt.ordinal();
    let week_of_year = dt.iso_week().week();

    let mut map = Map::new();
    map.insert("year".to_string(), Value::Number(dt.year().into()));
    map.insert(
        "month".to_string(),
        Value::String(month_abbr(dt.month()).to_string()),
    );
    map.insert("month_num".to_string(), Value::Number(dt.month().into()));
    map.insert("day".to_string(), Value::Number(dt.day().into()));
    map.insert(
        "weekday".to_string(),
        Value::String(weekday_abbr(dt.weekday()).to_string()),
    );
    map.insert("weekday_num".to_string(), Value::Number(weekday_num.into()));
    map.insert("hour".to_string(), Value::Number(hour_12.into()));
    map.insert("hour_24".to_string(), Value::Number(dt.hour().into()));
    map.insert("minute".to_string(), Value::Number(dt.minute().into()));
    map.insert("second".to_string(), Value::Number(dt.second().into()));
    map.insert("period".to_string(), Value::String(period.to_string()));
    map.insert("day_of_year".to_string(), Value::Number(day_of_year.into()));
    map.insert(
        "week_of_year".to_string(),
        Value::Number(week_of_year.into()),
    );
    map.insert(
        "iso".to_string(),
        Value::String(dt.format("%Y-%m-%dT%H:%M:%S").to_string()),
    );
    map
}

fn datetime_to_map_utc(dt: &DateTime<Utc>) -> Map<String, Value> {
    let hour_12 = {
        let h = dt.hour() % 12;
        if h == 0 { 12 } else { h }
    };
    let period = if dt.hour() < 12 { "AM" } else { "PM" };
    let weekday_num = dt.weekday().number_from_monday();
    let day_of_year = dt.ordinal();
    let week_of_year = dt.iso_week().week();

    let mut map = Map::new();
    map.insert("year".to_string(), Value::Number(dt.year().into()));
    map.insert(
        "month".to_string(),
        Value::String(month_abbr(dt.month()).to_string()),
    );
    map.insert("month_num".to_string(), Value::Number(dt.month().into()));
    map.insert("day".to_string(), Value::Number(dt.day().into()));
    map.insert(
        "weekday".to_string(),
        Value::String(weekday_abbr(dt.weekday()).to_string()),
    );
    map.insert("weekday_num".to_string(), Value::Number(weekday_num.into()));
    map.insert("hour".to_string(), Value::Number(hour_12.into()));
    map.insert("hour_24".to_string(), Value::Number(dt.hour().into()));
    map.insert("minute".to_string(), Value::Number(dt.minute().into()));
    map.insert("second".to_string(), Value::Number(dt.second().into()));
    map.insert("period".to_string(), Value::String(period.to_string()));
    map.insert("utc_offset".to_string(), Value::String("+0000".to_string()));
    map.insert("day_of_year".to_string(), Value::Number(day_of_year.into()));
    map.insert(
        "week_of_year".to_string(),
        Value::Number(week_of_year.into()),
    );
    map.insert("iso".to_string(), Value::String(dt.to_rfc3339()));
    map
}

impl Parser for TimestampParser {
    fn info(&self) -> &'static ParserInfo {
        &TIMESTAMP_INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        let input = input.trim();
        if input.is_empty() {
            return Err(ParseError::InvalidInput("empty input".to_string()));
        }

        // jc only uses first 10 chars (Unix timestamp, truncating floats)
        let ts_str = &input[..input.len().min(10)];
        let ts: i64 = ts_str.parse().map_err(|e| {
            ParseError::InvalidInput(format!("cannot parse timestamp '{}': {}", ts_str, e))
        })?;

        // naive = local time
        let dt_local = Local
            .timestamp_opt(ts, 0)
            .single()
            .ok_or_else(|| ParseError::Generic(format!("invalid timestamp: {}", ts)))?;
        let dt_utc = Utc
            .timestamp_opt(ts, 0)
            .single()
            .ok_or_else(|| ParseError::Generic(format!("invalid timestamp: {}", ts)))?;

        let naive_map = datetime_to_map_naive(&dt_local.naive_local());
        let utc_map = datetime_to_map_utc(&dt_utc);

        let mut map = Map::new();
        map.insert("naive".to_string(), Value::Object(naive_map));
        map.insert("utc".to_string(), Value::Object(utc_map));

        Ok(ParseOutput::Object(map))
    }
}

static TIMESTAMP_PARSER_INSTANCE: TimestampParser = TimestampParser;

inventory::submit! {
    ParserEntry::new(&TIMESTAMP_PARSER_INSTANCE)
}

#[cfg(test)]
mod tests {
    use super::*;
    use jc_rs_core::types::ParseOutput;

    fn parse_to_value(input: &str) -> serde_json::Value {
        let parser = TimestampParser;
        let result = parser.parse(input, false).unwrap();
        match result {
            ParseOutput::Object(map) => serde_json::Value::Object(map),
            _ => panic!("expected object"),
        }
    }

    #[test]
    fn test_timestamp_utc_fields() {
        let v = parse_to_value("1658599410");
        // UTC fields
        assert_eq!(v["utc"]["year"], 2022);
        assert_eq!(v["utc"]["month"], "Jul");
        assert_eq!(v["utc"]["day"], 23);
        assert_eq!(v["utc"]["hour_24"], 18);
        assert_eq!(v["utc"]["minute"], 3);
        assert_eq!(v["utc"]["second"], 30);
        assert_eq!(v["utc"]["utc_offset"], "+0000");
        // naive fields exist
        assert!(v["naive"]["year"].is_number());
    }

    #[test]
    fn test_timestamp_float_truncated() {
        // Should truncate to first 10 chars like jc does
        let v1 = parse_to_value("1658599410");
        let v2 = parse_to_value("1658599410.5");
        assert_eq!(v1["utc"]["year"], v2["utc"]["year"]);
    }
}
