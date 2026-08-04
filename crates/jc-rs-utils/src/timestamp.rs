//! Timestamp parsing utilities, ported from jc's `timestamp` class in utils.py.
//!
//! Supports 34+ datetime format strings with LRU caching and timezone normalization.

use chrono::{DateTime, Local, NaiveDateTime, TimeZone, Utc};
use lru::LruCache;
use std::collections::HashSet;
use std::num::NonZeroUsize;
use std::sync::Mutex;

/// Result of a timestamp parse operation.
#[derive(Debug, Clone, PartialEq)]
pub struct TimestampResult {
    /// Naive (local) epoch seconds. None if parsing failed.
    pub naive_epoch: Option<i64>,
    /// UTC epoch seconds (only set when UTC is detected in input). None otherwise.
    pub utc_epoch: Option<i64>,
    /// ISO 8601 string representation. UTC-aware if UTC was detected.
    pub iso: Option<String>,
}

/// A format entry with optional locale flag.
struct FmtEntry {
    /// Format string for chrono/strftime
    fmt: &'static str,
    /// If true, this format may need locale handling (we just try it anyway in Rust)
    _locale: bool,
}

/// All 34 supported datetime format strings, in the same order as jc's utils.py.
static FORMATS: &[FmtEntry] = &[
    FmtEntry {
        fmt: "%a %b %d %H:%M:%S %Y",
        _locale: false,
    }, // id 1000
    FmtEntry {
        fmt: "%a %b %d %H:%M:%S %Y %z",
        _locale: false,
    }, // id 1100
    FmtEntry {
        fmt: "%Y-%m-%dT%H:%M:%S.%f%Z",
        _locale: false,
    }, // id 1300
    FmtEntry {
        fmt: "%Y-%m-%dT%H:%M:%S.%f",
        _locale: false,
    }, // id 1310
    FmtEntry {
        fmt: "%b %d %Y %H:%M:%S.%f UTC",
        _locale: false,
    }, // id 1400
    FmtEntry {
        fmt: "%b %d %Y %H:%M:%S.%f",
        _locale: false,
    }, // id 1410
    FmtEntry {
        fmt: "%b %d %Y %H:%M:%S UTC",
        _locale: false,
    }, // id 1420
    FmtEntry {
        fmt: "%b %d %Y %H:%M:%S",
        _locale: false,
    }, // id 1430
    FmtEntry {
        fmt: "%Y-%m-%d %H:%M",
        _locale: false,
    }, // id 1500
    FmtEntry {
        fmt: "%m/%d/%Y %I:%M %p",
        _locale: false,
    }, // id 1600
    FmtEntry {
        fmt: "%m/%d/%Y, %I:%M:%S %p",
        _locale: false,
    }, // id 1700
    FmtEntry {
        fmt: "%m/%d/%Y, %I:%M:%S %p %Z",
        _locale: false,
    }, // id 1705
    FmtEntry {
        fmt: "%m/%d/%Y, %I:%M:%S %p UTC%z",
        _locale: false,
    }, // id 1710
    FmtEntry {
        fmt: "%A, %B %d, %Y %I:%M:%S %p",
        _locale: false,
    }, // id 1720
    FmtEntry {
        fmt: "%Y/%m/%d-%H:%M:%S.%f",
        _locale: false,
    }, // id 1750
    FmtEntry {
        fmt: "%Y/%m/%d-%H:%M:%S.%f%z",
        _locale: false,
    }, // id 1755
    FmtEntry {
        fmt: "%Y-%m-%d %H:%M:%S%z",
        _locale: false,
    }, // id 1760
    FmtEntry {
        fmt: "%d/%b/%Y:%H:%M:%S %z",
        _locale: false,
    }, // id 1800
    FmtEntry {
        fmt: "%a %d %b %Y %I:%M:%S %p %Z",
        _locale: false,
    }, // id 2000
    FmtEntry {
        fmt: "%a %d %b %Y %I:%M:%S %p",
        _locale: false,
    }, // id 3000
    FmtEntry {
        fmt: "%a %d %b %Y %I:%M:%S %p %z",
        _locale: false,
    }, // id 3100
    FmtEntry {
        fmt: "%a, %d %b %Y %H:%M:%S %Z",
        _locale: false,
    }, // id 3500
    FmtEntry {
        fmt: "%A %d %B %Y %I:%M:%S %p %Z",
        _locale: false,
    }, // id 4000
    FmtEntry {
        fmt: "%A %d %B %Y %I:%M:%S %p",
        _locale: false,
    }, // id 5000
    FmtEntry {
        fmt: "%a %b %d %I:%M:%S %p %Z %Y",
        _locale: false,
    }, // id 6000
    FmtEntry {
        fmt: "%a %b %d %H:%M:%S %Z %Y",
        _locale: false,
    }, // id 7000
    FmtEntry {
        fmt: "%b %d %H:%M:%S %Y",
        _locale: false,
    }, // id 7100
    FmtEntry {
        fmt: "%Y-%m-%d %H:%M:%S.%f %z",
        _locale: false,
    }, // id 7200
    FmtEntry {
        fmt: "%Y-%m-%d %H:%M:%S",
        _locale: false,
    }, // id 7250
    FmtEntry {
        fmt: "%Y-%m-%d %H:%M:%S %Z",
        _locale: false,
    }, // id 7255
    FmtEntry {
        fmt: "%a %Y-%m-%d %H:%M:%S %Z",
        _locale: false,
    }, // id 7300
    FmtEntry {
        fmt: "%a %d %b %Y %H:%M:%S %Z",
        _locale: true,
    }, // id 8000
    FmtEntry {
        fmt: "%a %d %b %Y %H:%M:%S",
        _locale: true,
    }, // id 8100
    FmtEntry {
        fmt: "%A %d %B %Y, %H:%M:%S UTC%z",
        _locale: true,
    }, // id 8200
    FmtEntry {
        fmt: "%A %d %B %Y, %H:%M:%S",
        _locale: true,
    }, // id 8300
];

/// Non-UTC timezone abbreviations to strip from datetime strings.
/// This list comes directly from jc's utils.py.
static TZ_ABBR: &[&str] = &[
    "A", "ACDT", "ACST", "ACT", "ACWST", "ADT", "AEDT", "AEST", "AET", "AFT", "AKDT", "AKST",
    "ALMT", "AMST", "AMT", "ANAST", "ANAT", "AQTT", "ART", "AST", "AT", "AWDT", "AWST", "AZOST",
    "AZOT", "AZST", "AZT", "AoE", "B", "BNT", "BOT", "BRST", "BRT", "BST", "BTT", "C", "CAST",
    "CAT", "CCT", "CDT", "CEST", "CET", "CHADT", "CHAST", "CHOST", "CHOT", "CHUT", "CIDST", "CIST",
    "CKT", "CLST", "CLT", "COT", "CST", "CT", "CVT", "CXT", "ChST", "D", "DAVT", "DDUT", "E",
    "EASST", "EAST", "EAT", "ECT", "EDT", "EEST", "EET", "EGST", "EGT", "EST", "ET", "F", "FET",
    "FJST", "FJT", "FKST", "FKT", "FNT", "G", "GALT", "GAMT", "GET", "GFT", "GILT", "GST", "GYT",
    "H", "HDT", "HKT", "HOVST", "HOVT", "HST", "I", "ICT", "IDT", "IOT", "IRDT", "IRKST", "IRKT",
    "IRST", "IST", "JST", "K", "KGT", "KOST", "KRAST", "KRAT", "KST", "KUYT", "L", "LHDT", "LHST",
    "LINT", "M", "MAGST", "MAGT", "MART", "MAWT", "MDT", "MHT", "MMT", "MSD", "MSK", "MST", "MT",
    "MUT", "MVT", "MYT", "N", "NCT", "NDT", "NFDT", "NFT", "NOVST", "NOVT", "NPT", "NRT", "NST",
    "NUT", "NZDT", "NZST", "O", "OMSST", "OMST", "ORAT", "P", "PDT", "PET", "PETST", "PETT", "PGT",
    "PHOT", "PHT", "PKT", "PMDT", "PMST", "PONT", "PST", "PT", "PWT", "PYST", "PYT", "Q", "QYZT",
    "R", "RET", "ROTT", "S", "SAKT", "SAMT", "SAST", "SBT", "SCT", "SGT", "SRET", "SRT", "SST",
    "SYOT", "T", "TAHT", "TFT", "TJT", "TKT", "TLT", "TMT", "TOST", "TOT", "TRT", "TVT", "U",
    "ULAST", "ULAT", "UYST", "UYT", "UZT", "V", "VET", "VLAST", "VLAT", "VOST", "VUT", "W", "WAKT",
    "WARST", "WAST", "WAT", "WEST", "WET", "WFT", "WGST", "WGT", "WIB", "WIT", "WITA", "WST", "WT",
    "X", "Y", "YAKST", "YAKT", "YAPT", "YEKST", "YEKT", "UTC-1200", "UTC-1100", "UTC-1000",
    "UTC-0930", "UTC-0900", "UTC-0800", "UTC-0700", "UTC-0600", "UTC-0500", "UTC-0400", "UTC-0300",
    "UTC-0230", "UTC-0200", "UTC-0100", "UTC+0100", "UTC+0200", "UTC+0300", "UTC+0400", "UTC+0430",
    "UTC+0500", "UTC+0530", "UTC+0545", "UTC+0600", "UTC+0630", "UTC+0700", "UTC+0800", "UTC+0845",
    "UTC+0900", "UTC+1000", "UTC+1030", "UTC+1100", "UTC+1200", "UTC+1300", "UTC+1345", "UTC+1400",
];

/// Non-UTC offset suffixes to strip.
static OFFSET_SUFFIXES: &[&str] = &[
    "-12:00", "-11:00", "-10:00", "-09:30", "-09:00", "-08:00", "-07:00", "-06:00", "-05:00",
    "-04:00", "-03:00", "-02:30", "-02:00", "-01:00", "+01:00", "+02:00", "+03:00", "+04:00",
    "+04:30", "+05:00", "+05:30", "+05:45", "+06:00", "+06:30", "+07:00", "+08:00", "+08:45",
    "+09:00", "+10:00", "+10:30", "+11:00", "+12:00", "+13:00", "+13:45", "+14:00",
];

// LRU cache: key = (input string, optional format hint), value = TimestampResult
type TimestampCache = Mutex<LruCache<(String, Option<String>), TimestampResult>>;
static CACHE: std::sync::OnceLock<TimestampCache> = std::sync::OnceLock::new();

fn cache() -> &'static TimestampCache {
    CACHE.get_or_init(|| Mutex::new(LruCache::new(NonZeroUsize::new(256).unwrap())))
}

/// Static HashSet of non-UTC timezone abbreviations, built once and reused.
fn tz_abbr_set() -> &'static HashSet<&'static str> {
    static SET: std::sync::OnceLock<HashSet<&'static str>> = std::sync::OnceLock::new();
    SET.get_or_init(|| TZ_ABBR.iter().copied().collect())
}

/// Static compiled regex for normalizing subsecond precision > 6 digits.
fn subsecond_re() -> &'static regex::Regex {
    static RE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    RE.get_or_init(|| regex::Regex::new(r"(:\d{2}:\d{2}\.\d{6})\d+").unwrap())
}

/// Normalize an input datetime string following jc's algorithm:
/// - Replace "Coordinated Universal Time" → "UTC"
/// - Replace "Z" → "UTC" (for ISO-8601 Zulu)
/// - Replace "GMT" → "UTC"
/// - Strip non-UTC timezone abbreviations
/// - Strip non-UTC offset suffixes
/// - Normalize >6 digit subseconds to 6 digits
/// - Returns (normalized_string, utc_tz: bool)
fn normalize_datetime_str(input: &str) -> (String, bool) {
    let mut data = input.to_string();

    data = data.replace("Coordinated Universal Time", "UTC");
    data = data.replace('Z', "UTC");
    data = data.replace("GMT", "UTC");

    let utc_tz = if data.contains("UTC") {
        if data.contains("UTC+") || data.contains("UTC-") {
            data.contains("UTC+0000") || data.contains("UTC-0000")
        } else {
            true
        }
    } else {
        data.contains("+0000")
            || data.contains("-0000")
            || data.contains("+00:00")
            || data.contains("-00:00")
    };

    // Fix +00:00 for parsing
    data = data.replace("+00:00", "+0000");

    // Remove parentheses
    data = data.replace(['(', ')'], "");

    // Strip non-UTC timezone abbreviations from tokens.
    // Use the lazily-initialized static HashSet to avoid re-building it on every call.
    {
        let set = tz_abbr_set();
        let filtered: Vec<&str> = data
            .split_whitespace()
            .filter(|t| !set.contains(*t))
            .collect();
        data = filtered.join(" ");
    }

    // Strip non-UTC offset suffixes from end
    for suffix in OFFSET_SUFFIXES {
        if data.ends_with(suffix) {
            data = data[..data.len() - suffix.len()].trim_end().to_string();
            break;
        }
    }

    // Normalize subseconds > 6 digits to 6 digits.
    // Use the lazily-initialized static compiled regex to avoid recompiling.
    data = subsecond_re().replace(&data, "$1").to_string();

    (data.trim().to_string(), utc_tz)
}

/// Try to parse a naive datetime from a string using the given format.
fn try_parse_naive(s: &str, fmt: &str) -> Option<NaiveDateTime> {
    NaiveDateTime::parse_from_str(s, fmt).ok()
}

/// Try to parse a timezone-aware datetime from a string using the given format.
/// Epoch for a naive (timezone-less) wall-clock reading, interpreted in the
/// LOCAL timezone.
///
/// This mirrors Python's `datetime.timestamp()` on a naive datetime, which is
/// what jc relies on: a command that prints "Jan  5 14:29:24" is printing local
/// time, so the epoch has to be resolved through `$TZ`. Treating it as UTC —
/// what this code did before — puts every timestamp-bearing parser out by the
/// UTC offset (8 h in winter, 7 h in summer for jc's own PST8PDT fixtures) and
/// silently corrupts `epoch`, `login_epoch`, `modify_time_epoch` and friends
/// across ~20 parsers.
///
/// DST edge cases follow Python's `fold=0` behaviour: for an ambiguous local
/// time (the repeated hour when clocks go back) take the earlier instant; for a
/// nonexistent one (the skipped hour) fall back to the UTC reading rather than
/// failing, since jc yields a value there too.
fn naive_local_epoch(dt_naive: &NaiveDateTime) -> i64 {
    match Local.from_local_datetime(dt_naive) {
        chrono::LocalResult::Single(dt) => dt.timestamp(),
        chrono::LocalResult::Ambiguous(earlier, _later) => earlier.timestamp(),
        chrono::LocalResult::None => dt_naive.and_utc().timestamp(),
    }
}

fn try_parse_aware(s: &str, fmt: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_str(s, fmt)
        .ok()
        .map(|dt| dt.with_timezone(&Utc))
}

/// Parse a datetime string into a `TimestampResult`.
///
/// `format_hint`: an optional format string to try first.
pub fn parse_timestamp(input: &str, format_hint: Option<&str>) -> TimestampResult {
    let cache_key = (input.to_string(), format_hint.map(|s| s.to_string()));

    // Check cache
    if let Ok(mut c) = cache().lock()
        && let Some(cached) = c.get(&cache_key)
    {
        return cached.clone();
    }

    let result = do_parse(input, format_hint);

    // Store in cache
    if let Ok(mut c) = cache().lock() {
        c.put(cache_key, result.clone());
    }

    result
}

fn do_parse(input: &str, format_hint: Option<&str>) -> TimestampResult {
    let (normalized, utc_tz) = normalize_datetime_str(input);

    // Build ordered format list: hint first (if provided and matches a known format),
    // then the rest in order.
    let mut fmt_list: Vec<&str> = Vec::new();

    if let Some(hint) = format_hint {
        fmt_list.push(hint);
    }

    for entry in FORMATS {
        // Don't add the hint again
        if format_hint != Some(entry.fmt) {
            fmt_list.push(entry.fmt);
        }
    }

    let mut naive_epoch: Option<i64> = None;
    let mut utc_epoch: Option<i64> = None;
    let mut iso: Option<String> = None;

    for fmt in &fmt_list {
        // Try aware parse first (handles %z, %Z with UTC)
        if let Some(dt_aware) = try_parse_aware(&normalized, fmt) {
            let naive_dt = dt_aware.naive_utc();
            naive_epoch = Some(naive_dt.and_utc().timestamp());
            if utc_tz {
                utc_epoch = Some(dt_aware.timestamp());
                iso = Some(dt_aware.to_rfc3339());
            } else {
                iso = Some(naive_dt.format("%Y-%m-%dT%H:%M:%S").to_string());
            }
            break;
        }

        // Try naive parse
        if let Some(dt_naive) = try_parse_naive(&normalized, fmt) {
            naive_epoch = Some(naive_local_epoch(&dt_naive));
            if utc_tz {
                let dt_utc = Utc.from_utc_datetime(&dt_naive);
                utc_epoch = Some(dt_utc.timestamp());
                iso = Some(dt_utc.to_rfc3339());
            } else {
                iso = Some(dt_naive.format("%Y-%m-%dT%H:%M:%S").to_string());
            }
            break;
        }
    }

    TimestampResult {
        naive_epoch,
        utc_epoch,
        iso,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_utc_iso() {
        // ISO Format with Z (UTC)
        let r = parse_timestamp("2003-10-11T22:14:15.003Z", None);
        assert!(r.naive_epoch.is_some());
        assert!(r.utc_epoch.is_some());
        assert_eq!(r.naive_epoch, r.utc_epoch);
    }

    #[test]
    fn test_parse_no_tz() {
        // No timezone — naive only
        let r = parse_timestamp("2021-03-23 00:14", None);
        assert!(r.naive_epoch.is_some());
        assert!(r.utc_epoch.is_none());
    }

    #[test]
    fn test_parse_utc_explicit() {
        let r = parse_timestamp("Wed Mar 24 11:11:30 UTC 2021", None);
        assert!(r.naive_epoch.is_some());
        assert!(r.utc_epoch.is_some());
    }

    #[test]
    fn test_parse_invalid() {
        let r = parse_timestamp("not a date", None);
        assert!(r.naive_epoch.is_none());
        assert!(r.utc_epoch.is_none());
        assert!(r.iso.is_none());
    }

    #[test]
    fn test_parse_with_gmt() {
        // GMT should be treated as UTC
        let r = parse_timestamp("Wed, 31 Jan 2024 00:39:28 GMT", None);
        assert!(r.naive_epoch.is_some());
        assert!(r.utc_epoch.is_some());
    }

    #[test]
    fn test_cache_works() {
        let r1 = parse_timestamp("2021-03-23 00:14", None);
        let r2 = parse_timestamp("2021-03-23 00:14", None);
        assert_eq!(r1, r2);
    }
}
