//! Parser for `date` command output.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use jc_rs_utils::{parse_timestamp, timestamp::formats};
use serde_json::{Map, Value};

pub struct DateParser;

static INFO: ParserInfo = ParserInfo {
    name: "date",
    argument: "--date",
    version: "2.6.0",
    description: "Converts `date` command output to JSON",
    author: "jc-rs contributors",
    author_email: "os@g1sw.com",
    compatible: &[Platform::Linux, Platform::Darwin, Platform::FreeBSD],
    tags: &[Tag::Command, Tag::Slurpable],
    magic_commands: &["date"],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static DATE_PARSER: DateParser = DateParser;

inventory::submit! {
    ParserEntry::new(&DATE_PARSER)
}

/// List of timezone abbreviations (same set as jc's date.py).
static TZ_ABBR: &[&str] = &[
    "A", "ACDT", "ACST", "ACT", "ACWST", "ADT", "AEDT", "AEST", "AET", "AFT", "AKDT", "AKST",
    "ALMT", "AMST", "AMT", "ANAST", "ANAT", "AQTT", "ART", "AST", "AT", "AWDT", "AWST", "AZOST",
    "AZOT", "AZST", "AZT", "AoE", "B", "BNT", "BOT", "BRST", "BRT", "BST", "BTT", "C", "CAST",
    "CAT", "CCT", "CDT", "CEST", "CET", "CHADT", "CHAST", "CHOST", "CHOT", "CHUT", "CIDST", "CIST",
    "CKT", "CLST", "CLT", "COT", "CST", "CT", "CVT", "CXT", "ChST", "D", "DAVT", "DDUT", "E",
    "EASST", "EAST", "EAT", "ECT", "EDT", "EEST", "EET", "EGST", "EGT", "EST", "ET", "F", "FET",
    "FJST", "FJT", "FKST", "FKT", "FNT", "G", "GALT", "GAMT", "GET", "GFT", "GILT", "GMT", "GST",
    "GYT", "H", "HDT", "HKT", "HOVST", "HOVT", "HST", "I", "ICT", "IDT", "IOT", "IRDT", "IRKST",
    "IRKT", "IRST", "IST", "JST", "K", "KGT", "KOST", "KRAST", "KRAT", "KST", "KUYT", "L", "LHDT",
    "LHST", "LINT", "M", "MAGST", "MAGT", "MART", "MAWT", "MDT", "MHT", "MMT", "MSD", "MSK", "MST",
    "MT", "MUT", "MVT", "MYT", "N", "NCT", "NDT", "NFDT", "NFT", "NOVST", "NOVT", "NPT", "NRT",
    "NST", "NUT", "NZDT", "NZST", "O", "OMSST", "OMST", "ORAT", "P", "PDT", "PET", "PETST", "PETT",
    "PGT", "PHOT", "PHT", "PKT", "PMDT", "PMST", "PONT", "PST", "PT", "PWT", "PYST", "PYT", "Q",
    "QYZT", "R", "RET", "ROTT", "S", "SAKT", "SAMT", "SAST", "SBT", "SCT", "SGT", "SRET", "SRT",
    "SST", "SYOT", "T", "TAHT", "TFT", "TJT", "TKT", "TLT", "TMT", "TOST", "TOT", "TRT", "TVT",
    "U", "ULAST", "ULAT", "UYST", "UYT", "UZT", "V", "VET", "VLAST", "VLAT", "VOST", "VUT", "W",
    "WAKT", "WARST", "WAST", "WAT", "WEST", "WET", "WFT", "WGST", "WGT", "WIB", "WIT", "WITA",
    "WST", "WT", "X", "Y", "YAKST", "YAKT", "YAPT", "YEKST", "YEKT", "Z", "UTC", "UTC-1200",
    "UTC-1100", "UTC-1000", "UTC-0930", "UTC-0900", "UTC-0800", "UTC-0700", "UTC-0600", "UTC-0500",
    "UTC-0400", "UTC-0300", "UTC-0230", "UTC-0200", "UTC-0100", "UTC+0000", "UTC-0000", "UTC+0100",
    "UTC+0200", "UTC+0300", "UTC+0400", "UTC+0430", "UTC+0500", "UTC+0530", "UTC+0545", "UTC+0600",
    "UTC+0630", "UTC+0700", "UTC+0800", "UTC+0845", "UTC+0900", "UTC+1000", "UTC+1030", "UTC+1100",
    "UTC+1200", "UTC+1300", "UTC+1345", "UTC+1400",
];

fn find_timezone(data: &str) -> Option<String> {
    let cleaned = data.replace(['(', ')'], "");
    for term in cleaned.split_whitespace() {
        if TZ_ABBR.contains(&term) {
            return Some(term.to_string());
        }
    }
    None
}

impl Parser for DateParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Object(Map::new()));
        }

        let data = input.trim();
        let tz = find_timezone(data);

        let ts = parse_timestamp(data, &[formats::F1000, formats::F6000, formats::F7000]);

        // Use UTC-aware datetime if available, else naive
        let epoch = ts.naive_epoch;
        let epoch_utc = ts.utc_epoch;
        let iso = ts.iso.clone().unwrap_or_default();
        let timezone_aware = epoch_utc.is_some();

        // Parse iso string to extract fields
        let mut obj = Map::new();

        // We need to parse the datetime fields. Parse from the iso string.
        use chrono::{DateTime, NaiveDateTime, Utc};

        let dt_opt: Option<(i32, u32, u32, u32, u32, u32, u32, String, Option<String>)> = {
            // Try UTC-aware first
            if let Ok(dt) = iso.parse::<DateTime<Utc>>() {
                Some((
                    dt.year(),
                    dt.month(),
                    dt.day(),
                    dt.weekday_num(),
                    dt.hour(),
                    dt.minute(),
                    dt.second(),
                    dt.format("%b").to_string(),
                    if timezone_aware {
                        Some(dt.format("%z").to_string())
                    } else {
                        None
                    },
                ))
            } else if let Ok(dt) = NaiveDateTime::parse_from_str(&iso, "%Y-%m-%dT%H:%M:%S") {
                let weekday_num = dt.weekday_num();
                Some((
                    dt.year(),
                    dt.month(),
                    dt.day(),
                    weekday_num,
                    dt.hour(),
                    dt.minute(),
                    dt.second(),
                    dt.format("%b").to_string(),
                    None,
                ))
            } else {
                None
            }
        };

        if let Some((
            year,
            month_num,
            day,
            weekday_num,
            hour_24,
            minute,
            second,
            month_str,
            utc_off,
        )) = dt_opt
        {
            let hour_12 = if hour_24 % 12 == 0 { 12 } else { hour_24 % 12 };
            let period = if hour_24 < 12 { "AM" } else { "PM" };
            let weekday_abbr = weekday_abbr_from_num(weekday_num);
            let day_of_year = day_of_year_from(year, month_num, day);
            let week_of_year = week_of_year_from(year, month_num, day);

            obj.insert("year".to_string(), Value::Number(year.into()));
            obj.insert("month".to_string(), Value::String(month_str));
            obj.insert(
                "month_num".to_string(),
                Value::Number((month_num as i64).into()),
            );
            obj.insert("day".to_string(), Value::Number((day as i64).into()));
            obj.insert("weekday".to_string(), Value::String(weekday_abbr));
            obj.insert(
                "weekday_num".to_string(),
                Value::Number((weekday_num as i64).into()),
            );
            obj.insert("hour".to_string(), Value::Number((hour_12 as i64).into()));
            obj.insert(
                "hour_24".to_string(),
                Value::Number((hour_24 as i64).into()),
            );
            obj.insert("minute".to_string(), Value::Number((minute as i64).into()));
            obj.insert("second".to_string(), Value::Number((second as i64).into()));
            obj.insert("period".to_string(), Value::String(period.to_string()));
            obj.insert(
                "timezone".to_string(),
                match &tz {
                    Some(t) => Value::String(t.clone()),
                    None => Value::Null,
                },
            );
            obj.insert(
                "utc_offset".to_string(),
                match utc_off {
                    Some(off) => Value::String(off),
                    None => Value::Null,
                },
            );
            obj.insert(
                "day_of_year".to_string(),
                Value::Number((day_of_year as i64).into()),
            );
            obj.insert(
                "week_of_year".to_string(),
                Value::Number((week_of_year as i64).into()),
            );
            obj.insert("iso".to_string(), Value::String(iso));
            obj.insert(
                "epoch".to_string(),
                match epoch {
                    Some(e) => Value::Number(e.into()),
                    None => Value::Null,
                },
            );
            obj.insert(
                "epoch_utc".to_string(),
                match epoch_utc {
                    Some(e) => Value::Number(e.into()),
                    None => Value::Null,
                },
            );
            obj.insert("timezone_aware".to_string(), Value::Bool(timezone_aware));
        }

        Ok(ParseOutput::Object(obj))
    }
}

fn weekday_abbr_from_num(num: u32) -> String {
    match num {
        1 => "Mon",
        2 => "Tue",
        3 => "Wed",
        4 => "Thu",
        5 => "Fri",
        6 => "Sat",
        7 => "Sun",
        _ => "Mon",
    }
    .to_string()
}

fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

fn day_of_year_from(year: i32, month: u32, day: u32) -> u32 {
    let days_in_month = [
        31,
        if is_leap_year(year) { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let mut doy = day;
    for days in days_in_month.iter().take(month as usize - 1) {
        doy += days;
    }
    doy
}

/// `strftime %W`: complete weeks since the year's first Monday, with the days
/// before it counting as week 0.
///
/// The formula needs the weekday of January 1st. The hand-rolled Sakamoto
/// version this replaces fed it the *given* month instead of January, so the
/// answer was wrong for most of the year.
fn week_of_year_from(year: i32, month: u32, day: u32) -> u32 {
    use chrono::Datelike;

    let Some(date) = chrono::NaiveDate::from_ymd_opt(year, month, day) else {
        return 0;
    };
    let Some(jan1) = chrono::NaiveDate::from_ymd_opt(year, 1, 1) else {
        return 0;
    };
    let offset = jan1.weekday().num_days_from_monday();
    (date.ordinal() - 1 + offset) / 7
}

trait WeekdayNum {
    fn weekday_num(&self) -> u32;
}

impl WeekdayNum for chrono::DateTime<chrono::Utc> {
    fn weekday_num(&self) -> u32 {
        use chrono::Datelike;
        self.weekday().number_from_monday()
    }
}

impl WeekdayNum for chrono::NaiveDateTime {
    fn weekday_num(&self) -> u32 {
        use chrono::Datelike;
        self.weekday().number_from_monday()
    }
}

trait DateFields {
    fn year(&self) -> i32;
    fn month(&self) -> u32;
    fn day(&self) -> u32;
    fn hour(&self) -> u32;
    fn minute(&self) -> u32;
    fn second(&self) -> u32;
}

macro_rules! impl_date_fields {
    ($t:ty) => {
        impl DateFields for $t {
            fn year(&self) -> i32 {
                chrono::Datelike::year(self)
            }
            fn month(&self) -> u32 {
                chrono::Datelike::month(self)
            }
            fn day(&self) -> u32 {
                chrono::Datelike::day(self)
            }
            fn hour(&self) -> u32 {
                chrono::Timelike::hour(self)
            }
            fn minute(&self) -> u32 {
                chrono::Timelike::minute(self)
            }
            fn second(&self) -> u32 {
                chrono::Timelike::second(self)
            }
        }
    };
}

impl_date_fields!(chrono::DateTime<chrono::Utc>);
impl_date_fields!(chrono::NaiveDateTime);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_date_pdt() {
        let input = include_str!("../../../../tests/fixtures/generic/date.out");
        let expected: serde_json::Value =
            serde_json::from_str(include_str!("../../../../tests/fixtures/generic/date.json"))
                .unwrap();
        let parser = DateParser;
        let result = parser.parse(input.trim(), false).unwrap();
        if let ParseOutput::Object(obj) = result {
            let got = serde_json::Value::Object(obj);
            // Check key fields
            assert_eq!(got["year"], expected["year"]);
            assert_eq!(got["month"], expected["month"]);
            assert_eq!(got["day"], expected["day"]);
            assert_eq!(got["hour"], expected["hour"]);
            assert_eq!(got["hour_24"], expected["hour_24"]);
            assert_eq!(got["minute"], expected["minute"]);
            assert_eq!(got["second"], expected["second"]);
            assert_eq!(got["timezone"], expected["timezone"]);
            assert_eq!(got["timezone_aware"], expected["timezone_aware"]);
            assert_eq!(got["period"], expected["period"]);
            assert_eq!(got["weekday"], expected["weekday"]);
        } else {
            panic!("Expected Object");
        }
    }

    #[test]
    fn test_date_utc_after_midnight() {
        let input = include_str!("../../../../tests/fixtures/generic/date-after-midnight.out");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/generic/date-after-midnight.json"
        ))
        .unwrap();
        let parser = DateParser;
        let result = parser.parse(input.trim(), false).unwrap();
        if let ParseOutput::Object(obj) = result {
            let got = serde_json::Value::Object(obj);
            assert_eq!(got["year"], expected["year"]);
            assert_eq!(got["month"], expected["month"]);
            assert_eq!(got["hour"], expected["hour"]);
            assert_eq!(got["hour_24"], expected["hour_24"]);
            assert_eq!(got["timezone"], expected["timezone"]);
            assert_eq!(got["timezone_aware"], expected["timezone_aware"]);
            assert_eq!(got["period"], expected["period"]);
        } else {
            panic!("Expected Object");
        }
    }

    #[test]
    fn test_date_empty() {
        let parser = DateParser;
        let result = parser.parse("", false).unwrap();
        if let ParseOutput::Object(obj) = result {
            assert!(obj.is_empty());
        } else {
            panic!("Expected Object");
        }
    }
}
