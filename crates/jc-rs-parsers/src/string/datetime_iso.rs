//! ISO 8601 Datetime string parser.

use chrono::{DateTime, Datelike, FixedOffset, NaiveDateTime, Timelike, Weekday};
use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use regex::Regex;
use serde_json::{Map, Value};
use std::sync::OnceLock;

struct DatetimeIsoParser;

static DATETIME_ISO_INFO: ParserInfo = ParserInfo {
    name: "datetime_iso",
    argument: "--datetime-iso",
    version: "1.0.0",
    description: "ISO 8601 Datetime string parser",
    author: "jc-rs contributors",
    author_email: "jc-rs@example.com",
    compatible: &[Platform::Universal],
    tags: &[Tag::String],
    magic_commands: &[],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static ISO8601_RE: OnceLock<Regex> = OnceLock::new();

fn get_iso8601_re() -> &'static Regex {
    ISO8601_RE.get_or_init(|| {
        // ISO 8601 datetime pattern: YYYY-MM-DDTHH:MM:SS.ffffff±HH:MM or Z
        Regex::new(
            r"^(?P<year>[0-9]{4})(?:-(?P<monthdash>[0-9]{1,2})(?:-(?P<daydash>[0-9]{1,2})(?:[T ](?P<hour>[0-9]{2})(?::(?P<minute>[0-9]{2})(?::(?P<second>[0-9]{1,2})(?:[.,](?P<second_fraction>[0-9]+))?)?)?(?P<timezone>Z|(?P<tz_sign>[+-])(?P<tz_hour>[0-9]{2}):?(?P<tz_minute>[0-9]{2})?))?)?)?$"
        )
        .expect("iso8601 regex compile error")
    })
}

fn parse_iso8601(input: &str) -> Result<DateTime<FixedOffset>, ParseError> {
    let re = get_iso8601_re();
    let caps = re.captures(input).ok_or_else(|| {
        ParseError::InvalidInput(format!("cannot parse ISO 8601 datetime: '{}'", input))
    })?;

    let year: i32 = caps["year"]
        .parse()
        .map_err(|e| ParseError::Generic(format!("{}", e)))?;
    let month_str = caps.name("monthdash").or_else(|| caps.name("month"));
    let month: u32 = month_str
        .map(|m| m.as_str().parse().unwrap_or(1))
        .unwrap_or(1);
    let day_str = caps.name("daydash").or_else(|| caps.name("day"));
    let day: u32 = day_str
        .map(|d| d.as_str().parse().unwrap_or(1))
        .unwrap_or(1);
    let hour: u32 = caps
        .name("hour")
        .map(|h| h.as_str().parse().unwrap_or(0))
        .unwrap_or(0);
    let minute: u32 = caps
        .name("minute")
        .map(|m| m.as_str().parse().unwrap_or(0))
        .unwrap_or(0);
    let second: u32 = caps
        .name("second")
        .map(|s| s.as_str().parse().unwrap_or(0))
        .unwrap_or(0);
    let microsecond: u32 = caps
        .name("second_fraction")
        .map(|sf| {
            let frac = sf.as_str();
            // Normalize to 6 digits (microseconds)
            let padded = format!("{:0<6}", &frac[..frac.len().min(6)]);
            padded.parse().unwrap_or(0)
        })
        .unwrap_or(0);

    // Timezone
    let tz_offset_secs: i32 = if let Some(tz) = caps.name("timezone") {
        let tz_str = tz.as_str();
        if tz_str == "Z" {
            0
        } else {
            let sign: i32 = if caps.name("tz_sign").map(|s| s.as_str()) == Some("-") {
                -1
            } else {
                1
            };
            let tz_hour: i32 = caps
                .name("tz_hour")
                .map(|h| h.as_str().parse().unwrap_or(0))
                .unwrap_or(0);
            let tz_min: i32 = caps
                .name("tz_minute")
                .map(|m| m.as_str().parse().unwrap_or(0))
                .unwrap_or(0);
            sign * (tz_hour * 3600 + tz_min * 60)
        }
    } else {
        0 // Default to UTC if no timezone
    };

    let offset = FixedOffset::east_opt(tz_offset_secs).ok_or_else(|| {
        ParseError::Generic(format!("invalid timezone offset: {}", tz_offset_secs))
    })?;

    let naive = NaiveDateTime::new(
        chrono::NaiveDate::from_ymd_opt(year, month, day).ok_or_else(|| {
            ParseError::InvalidInput(format!("invalid date: {}-{}-{}", year, month, day))
        })?,
        chrono::NaiveTime::from_hms_micro_opt(hour, minute, second, microsecond).ok_or_else(
            || {
                ParseError::InvalidInput(format!(
                    "invalid time: {}:{}:{}.{}",
                    hour, minute, second, microsecond
                ))
            },
        )?,
    );

    Ok(DateTime::from_naive_utc_and_offset(
        naive - chrono::Duration::seconds(tz_offset_secs as i64),
        offset,
    ))
}

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

fn day_of_year(dt: &DateTime<FixedOffset>) -> u32 {
    dt.ordinal()
}

fn week_of_year(dt: &DateTime<FixedOffset>) -> u32 {
    // ISO week number per strftime %W (week starting Monday, first week contains Jan 1)
    // Use chrono's week calculation
    dt.iso_week().week()
}

impl Parser for DatetimeIsoParser {
    fn info(&self) -> &'static ParserInfo {
        &DATETIME_ISO_INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        let input = input.trim();
        if input.is_empty() {
            return Err(ParseError::InvalidInput("empty input".to_string()));
        }

        let dt = parse_iso8601(input)?;

        let hour_12 = {
            let h = dt.hour() % 12;
            if h == 0 { 12 } else { h }
        };
        let period = if dt.hour() < 12 { "AM" } else { "PM" };
        let utc_offset = {
            let secs = dt.offset().local_minus_utc();
            let sign = if secs >= 0 { '+' } else { '-' };
            let abs_secs = secs.unsigned_abs();
            format!(
                "{}{:02}{:02}",
                sign,
                abs_secs / 3600,
                (abs_secs % 3600) / 60
            )
        };
        let timestamp = dt.timestamp();
        let weekday_num = dt.weekday().number_from_monday(); // 1=Mon..7=Sun (ISO)

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
        map.insert(
            "microsecond".to_string(),
            Value::Number((dt.timestamp_subsec_micros()).into()),
        );
        map.insert("period".to_string(), Value::String(period.to_string()));
        map.insert("utc_offset".to_string(), Value::String(utc_offset));
        map.insert(
            "day_of_year".to_string(),
            Value::Number(day_of_year(&dt).into()),
        );
        map.insert(
            "week_of_year".to_string(),
            Value::Number(week_of_year(&dt).into()),
        );
        map.insert("iso".to_string(), Value::String(dt.to_rfc3339()));
        map.insert("timestamp".to_string(), Value::Number(timestamp.into()));

        Ok(ParseOutput::Object(map))
    }
}

static DATETIME_ISO_PARSER_INSTANCE: DatetimeIsoParser = DatetimeIsoParser;

inventory::submit! {
    ParserEntry::new(&DATETIME_ISO_PARSER_INSTANCE)
}

#[cfg(test)]
mod tests {
    use super::*;
    use jc_rs_core::types::ParseOutput;

    fn parse_to_value(input: &str) -> serde_json::Value {
        let parser = DatetimeIsoParser;
        let result = parser.parse(input, false).unwrap();
        match result {
            ParseOutput::Object(map) => serde_json::Value::Object(map),
            _ => panic!("expected object"),
        }
    }

    #[test]
    fn test_datetime_iso_basic() {
        let v = parse_to_value("2022-07-20T14:52:45Z");
        assert_eq!(v["year"], 2022);
        assert_eq!(v["month"], "Jul");
        assert_eq!(v["month_num"], 7);
        assert_eq!(v["day"], 20);
        assert_eq!(v["weekday"], "Wed");
        assert_eq!(v["hour_24"], 14);
        assert_eq!(v["minute"], 52);
        assert_eq!(v["second"], 45);
        assert_eq!(v["timestamp"], 1658328765_i64);
    }

    #[test]
    fn test_datetime_iso_period() {
        let v = parse_to_value("2022-07-20T14:52:45Z");
        assert_eq!(v["period"], "PM");
        assert_eq!(v["hour"], 2); // 2 PM in 12-hour
    }

    #[test]
    fn test_datetime_iso_utc_offset() {
        let v = parse_to_value("2022-07-20T14:52:45Z");
        assert_eq!(v["utc_offset"], "+0000");
    }
}
