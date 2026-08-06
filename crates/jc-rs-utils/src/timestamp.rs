//! Timestamp parsing utilities, ported from jc's `timestamp` class in utils.py.
//!
//! Supports 34+ datetime format strings with LRU caching and timezone normalization.

use chrono::{DateTime, FixedOffset, Local, NaiveDateTime, TimeZone, Utc};
use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::HashSet;

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

/// jc's datetime formats, addressed by the same numeric ids jc uses so a call
/// site here can be read against the jc parser it mirrors.
///
/// A parser passing the right hint matters for more than speed: hinted formats
/// are tried *first*, so for a string two formats can both parse, the hint
/// decides which one wins, exactly as in jc.
pub mod formats {
    pub const F1000: &str = "%a %b %d %H:%M:%S %Y";
    pub const F1100: &str = "%a %b %d %H:%M:%S %Y %z";
    pub const F1300: &str = "%Y-%m-%dT%H:%M:%S.%f%Z";
    pub const F1310: &str = "%Y-%m-%dT%H:%M:%S.%f";
    pub const F1400: &str = "%b %d %Y %H:%M:%S.%f UTC";
    pub const F1410: &str = "%b %d %Y %H:%M:%S.%f";
    pub const F1420: &str = "%b %d %Y %H:%M:%S UTC";
    pub const F1430: &str = "%b %d %Y %H:%M:%S";
    pub const F1500: &str = "%Y-%m-%d %H:%M";
    pub const F1600: &str = "%m/%d/%Y %I:%M %p";
    pub const F1700: &str = "%m/%d/%Y, %I:%M:%S %p";
    pub const F1705: &str = "%m/%d/%Y, %I:%M:%S %p %Z";
    pub const F1710: &str = "%m/%d/%Y, %I:%M:%S %p UTC%z";
    pub const F1720: &str = "%A, %B %d, %Y %I:%M:%S %p";
    pub const F1750: &str = "%Y/%m/%d-%H:%M:%S.%f";
    pub const F1755: &str = "%Y/%m/%d-%H:%M:%S.%f%z";
    pub const F1760: &str = "%Y-%m-%d %H:%M:%S%z";
    pub const F1800: &str = "%d/%b/%Y:%H:%M:%S %z";
    pub const F2000: &str = "%a %d %b %Y %I:%M:%S %p %Z";
    pub const F3000: &str = "%a %d %b %Y %I:%M:%S %p";
    pub const F3100: &str = "%a %d %b %Y %I:%M:%S %p %z";
    pub const F3500: &str = "%a, %d %b %Y %H:%M:%S %Z";
    pub const F4000: &str = "%A %d %B %Y %I:%M:%S %p %Z";
    pub const F5000: &str = "%A %d %B %Y %I:%M:%S %p";
    pub const F6000: &str = "%a %b %d %I:%M:%S %p %Z %Y";
    pub const F7000: &str = "%a %b %d %H:%M:%S %Z %Y";
    pub const F7100: &str = "%b %d %H:%M:%S %Y";
    pub const F7200: &str = "%Y-%m-%d %H:%M:%S.%f %z";
    pub const F7250: &str = "%Y-%m-%d %H:%M:%S";
    pub const F7255: &str = "%Y-%m-%d %H:%M:%S %Z";
    pub const F7300: &str = "%a %Y-%m-%d %H:%M:%S %Z";
    pub const F8000: &str = "%a %d %b %Y %H:%M:%S %Z";
    pub const F8100: &str = "%a %d %b %Y %H:%M:%S";
    pub const F8200: &str = "%A %d %B %Y, %H:%M:%S UTC%z";
    pub const F8300: &str = "%A %d %B %Y, %H:%M:%S";
}

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

/// Memo for [`parse_timestamp`], as a direct-mapped table rather than an LRU.
///
/// The LRU it replaces had to own its key, so every call — hit or miss — copied
/// the input into a fresh `String` before it could even ask the question, and
/// took a process-wide `Mutex` twice to do it. A slot holds its own key, so a
/// lookup compares against the caller's `&str` and allocates nothing on a hit;
/// a collision simply evicts, which for the access pattern here (a handful of
/// distinct stamps repeated across thousands of records) costs nothing an LRU
/// would have saved. The table is per-thread because the function is pure:
/// threads agreeing on the answer is all the sharing was ever buying.
struct CacheSlot {
    input: String,
    hints: &'static [&'static str],
    result: TimestampResult,
}

const CACHE_SLOTS: usize = 512;

// Boxed so the table itself is 512 pointers — 4 KiB, which stays in L1 — and
// not 512 inline slots, which at ~100 bytes each would evict on every probe.
thread_local! {
    static CACHE: RefCell<Vec<Option<Box<CacheSlot>>>> = const { RefCell::new(Vec::new()) };
}

/// FNV-1a over the input, mixed with the hint list's identity.
fn cache_slot_for(input: &str, hints: &[&str]) -> usize {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325 ^ (hints.len() as u64);
    if let Some(first) = hints.first() {
        h ^= first.len() as u64;
    }
    for &b in input.as_bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    (h as usize) % CACHE_SLOTS
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

/// True when `s.split_whitespace().collect::<Vec<_>>().join(" ")` would hand
/// back `s` unchanged: ASCII, no leading or trailing space, no runs, no tabs.
///
/// Non-ASCII input answers `false` rather than reason about Unicode
/// `White_Space`, since a datetime string is never non-ASCII in practice.
fn is_single_spaced(s: &str) -> bool {
    if !s.is_ascii() {
        return false;
    }
    let bytes = s.as_bytes();
    if bytes.first().is_some_and(u8::is_ascii_whitespace)
        || bytes.last().is_some_and(u8::is_ascii_whitespace)
    {
        return false;
    }
    let mut prev_space = false;
    for &c in bytes {
        if c.is_ascii_whitespace() {
            if c != b' ' || prev_space {
                return false;
            }
            prev_space = true;
        } else {
            prev_space = false;
        }
    }
    true
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
    // Each step rewrites the string only when it has something to rewrite. A
    // typical stamp matches none of them, and used to pay eight allocations to
    // discover that.
    let mut data: Cow<'_, str> = Cow::Borrowed(input);

    if data.contains("Coordinated Universal Time") {
        data = Cow::Owned(data.replace("Coordinated Universal Time", "UTC"));
    }
    if data.contains('Z') {
        data = Cow::Owned(data.replace('Z', "UTC"));
    }
    if data.contains("GMT") {
        data = Cow::Owned(data.replace("GMT", "UTC"));
    }

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
    if data.contains("+00:00") {
        data = Cow::Owned(data.replace("+00:00", "+0000"));
    }

    // Remove parentheses
    if data.contains(['(', ')']) {
        data = Cow::Owned(data.replace(['(', ')'], ""));
    }

    // Strip non-UTC timezone abbreviations from tokens.
    // Use the lazily-initialized static HashSet to avoid re-building it on every call.
    {
        let set = tz_abbr_set();
        // The split/join only has to happen when a token is actually going to
        // be dropped, or when the whitespace itself needs normalising — which
        // for a datetime string is neither, nearly always.
        let needs_rebuild =
            !is_single_spaced(&data) || data.split_whitespace().any(|t| set.contains(t));
        if needs_rebuild {
            let filtered: Vec<&str> = data
                .split_whitespace()
                .filter(|t| !set.contains(*t))
                .collect();
            data = Cow::Owned(filtered.join(" "));
        }
    }

    // Strip non-UTC offset suffixes from end
    for suffix in OFFSET_SUFFIXES {
        if data.ends_with(suffix) {
            let cut = data.len() - suffix.len();
            data = Cow::Owned(data[..cut].trim_end().to_string());
            break;
        }
    }

    // Normalize subseconds > 6 digits to 6 digits.
    // Use the lazily-initialized static compiled regex to avoid recompiling.
    if let Cow::Owned(replaced) = subsecond_re().replace(&data, "$1") {
        data = Cow::Owned(replaced);
    }

    match data {
        // The final `trim` is almost always a no-op, and when the string is
        // already owned there is no reason to copy it a ninth time.
        Cow::Owned(mut s) => {
            let trimmed = s.trim();
            if trimmed.len() != s.len() {
                s = trimmed.to_string();
            }
            (s, utc_tz)
        }
        Cow::Borrowed(s) => (s.trim().to_string(), utc_tz),
    }
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
/// time, so the epoch has to be resolved through `$TZ`. Treating it as UTC,
/// which is what this code did before, puts every timestamp-bearing parser out
/// by the UTC offset (8 h in winter, 7 h in summer for jc's own PST8PDT
/// fixtures) and silently corrupts `epoch`, `login_epoch`, `modify_time_epoch`
/// and friends across ~20 parsers.
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

/// Parse a string whose format carries an offset (`%z`), keeping that offset
/// rather than normalising to UTC, because the caller needs the wall clock as
/// written. See [`do_parse`] for why.
fn try_parse_aware(s: &str, fmt: &str) -> Option<DateTime<FixedOffset>> {
    DateTime::parse_from_str(s, fmt).ok()
}

/// Parse a datetime string into a `TimestampResult`.
///
/// `format_hint`: an optional format string to try first.
pub fn parse_timestamp(input: &str, hints: &'static [&'static str]) -> TimestampResult {
    let slot = cache_slot_for(input, hints);

    let hit = CACHE.with(|c| {
        let mut table = c.borrow_mut();
        if table.is_empty() {
            table.resize_with(CACHE_SLOTS, || None);
        }
        match &table[slot] {
            Some(entry) if entry.input == input && entry.hints == hints => {
                Some(entry.result.clone())
            }
            _ => None,
        }
    });
    if let Some(result) = hit {
        return result;
    }

    let result = do_parse(input, hints);

    CACHE.with(|c| {
        let mut table = c.borrow_mut();
        table[slot] = Some(Box::new(CacheSlot {
            input: input.to_string(),
            hints,
            result: result.clone(),
        }));
    });

    result
}

fn do_parse(input: &str, hints: &[&str]) -> TimestampResult {
    let (normalized, utc_tz) = normalize_datetime_str(input);

    // Hinted formats first, then the rest in jc's order. Without a hint every
    // parse walks the whole table until something sticks, which for a format
    // near the end of the list is ~30 failed strptime attempts per record.
    // Chained lazily: the list was materialised into a `Vec` on every call, and
    // the first candidate is the right one whenever a hint was given.
    let fmt_list = hints.iter().copied().chain(
        FORMATS
            .iter()
            .map(|entry| entry.fmt)
            .filter(|fmt| !hints.contains(fmt)),
    );

    let mut naive_epoch: Option<i64> = None;
    let mut utc_epoch: Option<i64> = None;
    let mut iso: Option<String> = None;

    for fmt in fmt_list {
        // Try aware parse first (handles %z, %Z with UTC)
        if let Some(dt_aware) = try_parse_aware(&normalized, fmt) {
            // jc computes the naive stamp as `dt.replace(tzinfo=None).timestamp()`:
            // the parsed offset is *discarded* and the wall clock as written is
            // read in the local zone. Converting to UTC instead, which is the
            // intuitive reading, puts every `epoch` field out by the
            // difference between the two zones (three hours for `git log`'s
            // -0400 stamps under the corpus's PST8PDT).
            let wall_clock = dt_aware.naive_local();
            naive_epoch = Some(naive_local_epoch(&wall_clock));
            if utc_tz {
                let dt_utc = Utc.from_utc_datetime(&wall_clock);
                utc_epoch = Some(dt_utc.timestamp());
                iso = Some(dt_utc.to_rfc3339());
            } else {
                iso = Some(wall_clock.format("%Y-%m-%dT%H:%M:%S").to_string());
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
        // `naive` and `utc` are two different readings of the same wall clock
        // and are equal only where the local offset is zero. jc, under its own
        // TZ=PST8PDT, reports:
        //
        //   >>> t = jc.utils.timestamp('2003-10-11T22:14:15.003Z', None)
        //   >>> t.naive, t.utc
        //   (1065935655, 1065910455)
        //
        // naive reads 22:14:15 as local time, utc reads it as UTC; the 25200 s
        // gap is PDT's offset. An earlier version of this test asserted the two
        // were equal, which is what a UTC-everywhere implementation produces,
        // and that bug put every *_epoch field in the corpus out by the offset.
        let r = parse_timestamp("2003-10-11T22:14:15.003Z", &[]);
        assert_eq!(r.utc_epoch, Some(1065910455));
        assert_eq!(
            r.naive_epoch.expect("naive epoch") - r.utc_epoch.expect("utc epoch"),
            local_utc_offset_at(1065910455),
            "naive must be the local-time reading of the same wall clock"
        );
    }

    /// Seconds that local time is ahead of UTC at the given instant, so the
    /// test states the relationship rather than hard-coding one timezone.
    fn local_utc_offset_at(epoch: i64) -> i64 {
        use chrono::Offset;
        let dt = DateTime::from_timestamp(epoch, 0).expect("valid epoch");
        -(Local
            .offset_from_utc_datetime(&dt.naive_utc())
            .fix()
            .local_minus_utc() as i64)
    }

    #[test]
    fn test_parse_no_tz() {
        // No timezone: naive only
        let r = parse_timestamp("2021-03-23 00:14", &[]);
        assert!(r.naive_epoch.is_some());
        assert!(r.utc_epoch.is_none());
    }

    #[test]
    fn test_parse_utc_explicit() {
        let r = parse_timestamp("Wed Mar 24 11:11:30 UTC 2021", &[]);
        assert!(r.naive_epoch.is_some());
        assert!(r.utc_epoch.is_some());
    }

    #[test]
    fn test_parse_invalid() {
        let r = parse_timestamp("not a date", &[]);
        assert!(r.naive_epoch.is_none());
        assert!(r.utc_epoch.is_none());
        assert!(r.iso.is_none());
    }

    #[test]
    fn test_parse_with_gmt() {
        // GMT should be treated as UTC
        let r = parse_timestamp("Wed, 31 Jan 2024 00:39:28 GMT", &[]);
        assert!(r.naive_epoch.is_some());
        assert!(r.utc_epoch.is_some());
    }

    #[test]
    fn test_cache_works() {
        let r1 = parse_timestamp("2021-03-23 00:14", &[]);
        let r2 = parse_timestamp("2021-03-23 00:14", &[]);
        assert_eq!(r1, r2);
    }
}
