//! Conversion utilities ported from jc's utils.py.

use regex::Regex;
use std::borrow::Cow;
use std::sync::OnceLock;

fn size_token_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(\d+(?:\.\d+)?)").expect("valid size token pattern"))
}

/// True for the characters jc's `[^0-9\-\.]` regex *keeps*.
#[inline]
const fn is_numeric_byte(b: u8) -> bool {
    b.is_ascii_digit() || b == b'-' || b == b'.'
}

/// Drop every character jc's `[^0-9\-\.]` would strip.
///
/// This runs on every numeric field of every record — 89 call sites feed it —
/// and the overwhelmingly common input is already clean, so the clean case
/// borrows instead of allocating. The discarded set is a superset of the
/// non-ASCII range, so filtering bytes and rebuilding is exact: every byte kept
/// is ASCII, and every continuation byte of a multi-byte char is >= 0x80 and so
/// dropped, which is what the Unicode-aware regex did too.
fn strip_non_numeric(value: &str) -> Cow<'_, str> {
    if value.bytes().all(is_numeric_byte) {
        return Cow::Borrowed(value);
    }
    let mut out = String::with_capacity(value.len());
    for b in value.bytes() {
        if is_numeric_byte(b) {
            out.push(b as char);
        }
    }
    Cow::Owned(out)
}

/// Convert a string to i64 by stripping non-numeric characters.
///
/// Matches jc's `convert_to_int`: strips everything except digits, `-`, `.`,
/// tries int parse first, then float-to-int.
pub fn convert_to_int(value: &str) -> Option<i64> {
    let cleaned = strip_non_numeric(value);
    let s = cleaned.as_ref();
    if s.is_empty() {
        return None;
    }
    if let Ok(i) = s.parse::<i64>() {
        return Some(i);
    }
    if let Ok(f) = s.parse::<f64>() {
        return Some(f as i64);
    }
    None
}

/// Convert a string to f64 by stripping non-numeric characters.
///
/// Matches jc's `convert_to_float`.
pub fn convert_to_float(value: &str) -> Option<f64> {
    let cleaned = strip_non_numeric(value);
    let s = cleaned.as_ref();
    if s.is_empty() {
        return None;
    }
    s.parse::<f64>().ok()
}

/// Convert a string to bool using jc's truthy/falsy rules.
///
/// Truthy strings: "y", "yes", "true", "*" (case-insensitive).
/// If the string parses as a float, uses numeric truthiness.
/// Empty string → false. Unrecognized → None.
pub fn convert_to_bool(value: &str) -> Option<bool> {
    // Try numeric first
    if let Some(f) = convert_to_float(value) {
        return Some(f != 0.0);
    }
    // Non-numeric string. The candidates are all ASCII, so a case-insensitive
    // ASCII compare answers the same question as lowercasing the input first,
    // without the allocation.
    if value.is_empty() {
        return Some(false);
    }
    if ["y", "yes", "true", "*"]
        .iter()
        .any(|t| value.eq_ignore_ascii_case(t))
    {
        return Some(true);
    }
    if ["n", "no", "false", "0"]
        .iter()
        .any(|t| value.eq_ignore_ascii_case(t))
    {
        return Some(false);
    }
    None
}

/// Parse a human-readable size string like "10KB", "5.2 MiB" into bytes.
///
/// `binary_mode`: treat ambiguous units (KB, MB, etc.) as binary (1024-based).
///
/// Mirrors jc's `convert_size_to_int` with `binary` parameter.
pub fn convert_size_to_int(size: &str, binary_mode: bool) -> Option<i64> {
    // Remove commas
    let size: Cow<'_, str> = if size.contains(',') {
        Cow::Owned(size.replace(',', ""))
    } else {
        Cow::Borrowed(size)
    };
    let size = size.trim();

    // Tokenize: split on digit sequences. The pattern is compiled once for the
    // process; building it per call cost more than the rest of this function by
    // three orders of magnitude, on all 20 call sites.
    let mut tokens: Vec<&str> = Vec::new();
    let mut last_end = 0;
    for m in size_token_re().find_iter(size) {
        let before = size[last_end..m.start()].trim();
        if !before.is_empty() {
            tokens.push(before);
        }
        tokens.push(m.as_str());
        last_end = m.end();
    }
    let remainder = size[last_end..].trim();
    if !remainder.is_empty() {
        tokens.push(remainder);
    }

    if tokens.is_empty() {
        return None;
    }

    // First token must be a number
    let num: f64 = tokens[0].parse().ok()?;

    // Get unit token if present
    let unit = if tokens.len() >= 2 {
        tokens[1].to_lowercase()
    } else {
        String::new()
    };

    // No unit or bytes
    if unit.is_empty() || unit.starts_with('b') {
        return Some(num as i64);
    }

    // Strip trailing 's' for plurals
    let unit = unit.trim_end_matches('s').to_string();

    struct SizeUnit {
        decimal_div: f64,
        binary_div: f64,
        symbol_dec: &'static str,
        name_dec: &'static str,
        symbol_bin: &'static str,
        name_bin: &'static str,
    }

    let units = [
        SizeUnit {
            decimal_div: 1e3,
            binary_div: 1024f64.powi(1),
            symbol_dec: "kb",
            name_dec: "kilobyte",
            symbol_bin: "kib",
            name_bin: "kibibyte",
        },
        SizeUnit {
            decimal_div: 1e6,
            binary_div: 1024f64.powi(2),
            symbol_dec: "mb",
            name_dec: "megabyte",
            symbol_bin: "mib",
            name_bin: "mebibyte",
        },
        SizeUnit {
            decimal_div: 1e9,
            binary_div: 1024f64.powi(3),
            symbol_dec: "gb",
            name_dec: "gigabyte",
            symbol_bin: "gib",
            name_bin: "gibibyte",
        },
        SizeUnit {
            decimal_div: 1e12,
            binary_div: 1024f64.powi(4),
            symbol_dec: "tb",
            name_dec: "terabyte",
            symbol_bin: "tib",
            name_bin: "tebibyte",
        },
        SizeUnit {
            decimal_div: 1e15,
            binary_div: 1024f64.powi(5),
            symbol_dec: "pb",
            name_dec: "petabyte",
            symbol_bin: "pib",
            name_bin: "pebibyte",
        },
        SizeUnit {
            decimal_div: 1e18,
            binary_div: 1024f64.powi(6),
            symbol_dec: "eb",
            name_dec: "exabyte",
            symbol_bin: "eib",
            name_bin: "exbibyte",
        },
        SizeUnit {
            decimal_div: 1e21,
            binary_div: 1024f64.powi(7),
            symbol_dec: "zb",
            name_dec: "zettabyte",
            symbol_bin: "zib",
            name_bin: "zebibyte",
        },
        SizeUnit {
            decimal_div: 1e24,
            binary_div: 1024f64.powi(8),
            symbol_dec: "yb",
            name_dec: "yottabyte",
            symbol_bin: "yib",
            name_bin: "yobibyte",
        },
    ];

    // Handle two-letter units ending in 'i' (Ki, Gi, etc.) → treat as binary (append 'b')
    let unit = if unit.len() == 2 && unit.ends_with('i') {
        format!("{}b", unit)
    } else {
        unit
    };

    for su in &units {
        // Binary units (KiB, MiB, etc.)
        if unit == su.symbol_bin || unit == su.name_bin {
            return Some((num * su.binary_div) as i64);
        }
        // Decimal/ambiguous: symbol (KB, MB..) or name (kilobyte..) or first letter match
        if unit == su.symbol_dec || unit == su.name_dec || unit.starts_with(&su.symbol_dec[..1]) {
            let div = if binary_mode {
                su.binary_div
            } else {
                su.decimal_div
            };
            return Some((num * div) as i64);
        }
    }

    None
}

/// Remove surrounding single or double quotes from a string.
///
/// If no matching quotes are found, returns the string unchanged.
pub fn remove_quotes(s: &str) -> String {
    if (s.starts_with('"') && s.ends_with('"')) || (s.starts_with('\'') && s.ends_with('\'')) {
        s[1..s.len() - 1].to_string()
    } else {
        s.to_string()
    }
}

/// Normalize a key: lowercase, replace special chars and spaces with `_`,
/// collapse multiple underscores, preserve leading underscore.
///
/// Matches jc's `normalize_key` exactly. Special chars include:
/// `!"#$%&'()*+,-./:;<=>?@[\]^{|}~ ` (and space).
pub fn normalize_key(key: &str) -> String {
    let trimmed = key.trim();
    if trimmed.is_ascii() {
        squash_key(trimmed)
    } else {
        // Only the Unicode path needs `to_lowercase`; `squash_key` then has
        // nothing left to fold. None of jc's special characters can be produced
        // by a lowercase mapping, so folding first is equivalent to folding
        // after the replacement, which is the order jc uses.
        squash_key(&trimmed.to_lowercase())
    }
}

/// jc's special set: `!"#$%&'()*+,-./:;<=>?@[\]^`{|}~` and space.
static SPECIAL_KEY_BYTE: [bool; 128] = {
    let mut t = [false; 128];
    let mut i = 0;
    while i < 128 {
        let b = i as u8;
        t[i] = matches!(
            b,
            b'!' | b'"'
                | b'#'
                | b'$'
                | b'%'
                | b'&'
                | b'\''
                | b'('
                | b')'
                | b'*'
                | b'+'
                | b','
                | b'-'
                | b'.'
                | b'/'
                | b':'
                | b';'
                | b'<'
                | b'='
                | b'>'
                | b'?'
                | b'@'
                | b'['
                | b'\\'
                | b']'
                | b'^'
                | b'`'
                | b'{'
                | b'|'
                | b'}'
                | b'~'
                | b' '
        );
        i += 1;
    }
    t
};

#[inline]
fn is_special_key_char(ch: char) -> bool {
    (ch as u32) < 128 && SPECIAL_KEY_BYTE[ch as usize]
}

/// Map the special set to `_`, lowercase, collapse runs of `_`, drop them from
/// both ends but keep a single leading one — in a single pass and a single
/// allocation.
///
/// The previous form ran 31 `String::replace` passes per key and then split and
/// rejoined, so it allocated 34 strings to produce one. Parsers call this once
/// per column *per row*.
fn squash_key(data: &str) -> String {
    let mut out = String::with_capacity(data.len());
    let mut first = true;
    // An underscore run is only written once something follows it.
    let mut pending = false;
    let mut wrote_any = false;

    for ch in data.chars() {
        if ch == '_' || is_special_key_char(ch) {
            if first {
                // jc keeps exactly one leading underscore ("%CPU" -> "_cpu").
                out.push('_');
            }
            pending = true;
        } else {
            if pending && wrote_any {
                out.push('_');
            }
            pending = false;
            out.push(ch.to_ascii_lowercase());
            wrote_any = true;
        }
        first = false;
    }

    out
}

/// Print a warning message to stderr. Respects `quiet` flag.
pub fn warning_message(lines: &[&str], quiet: bool) {
    if quiet || lines.is_empty() {
        return;
    }
    eprintln!("jc-rs:  Warning - {}", lines[0]);
    for line in &lines[1..] {
        if !line.is_empty() {
            eprintln!("               {}", line);
        }
    }
}

/// Print an error message to stderr.
pub fn error_message(lines: &[&str]) {
    if lines.is_empty() {
        return;
    }
    eprintln!("jc-rs:  Error - {}", lines[0]);
    for line in &lines[1..] {
        if !line.is_empty() {
            eprintln!("             {}", line);
        }
    }
}

/// Returns true if the input contains any non-whitespace characters.
pub fn has_data(input: &str) -> bool {
    !input.trim().is_empty()
}

/// Returns Err(InvalidInput) if the input is empty/whitespace-only.
pub fn input_type_check(input: &str) -> Result<(), jc_rs_core::error::ParseError> {
    if has_data(input) {
        Ok(())
    } else {
        Err(jc_rs_core::error::ParseError::InvalidInput(
            "Input data is empty or contains only whitespace.".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convert_to_int_basic() {
        assert_eq!(convert_to_int("42"), Some(42));
        assert_eq!(convert_to_int("-5"), Some(-5));
        assert_eq!(convert_to_int("3.7"), Some(3));
        assert_eq!(convert_to_int("abc"), None);
        assert_eq!(convert_to_int(""), None);
        assert_eq!(convert_to_int("10KB"), Some(10));
    }

    #[test]
    // `3.14` here is test data, not an approximation of pi.
    #[allow(clippy::approx_constant)]
    fn test_convert_to_float_basic() {
        assert_eq!(convert_to_float("3.14"), Some(3.14));
        assert_eq!(convert_to_float("abc"), None);
        assert_eq!(convert_to_float(""), None);
    }

    #[test]
    fn test_convert_to_bool_truthy() {
        assert_eq!(convert_to_bool("y"), Some(true));
        assert_eq!(convert_to_bool("yes"), Some(true));
        assert_eq!(convert_to_bool("true"), Some(true));
        assert_eq!(convert_to_bool("*"), Some(true));
        assert_eq!(convert_to_bool("Y"), Some(true));
        assert_eq!(convert_to_bool("YES"), Some(true));
        assert_eq!(convert_to_bool("True"), Some(true));
        assert_eq!(convert_to_bool("1"), Some(true));
    }

    #[test]
    fn test_convert_to_bool_falsy() {
        assert_eq!(convert_to_bool("n"), Some(false));
        assert_eq!(convert_to_bool("no"), Some(false));
        assert_eq!(convert_to_bool("false"), Some(false));
        assert_eq!(convert_to_bool("0"), Some(false));
        assert_eq!(convert_to_bool(""), Some(false));
    }

    #[test]
    fn test_convert_to_bool_numeric() {
        assert_eq!(convert_to_bool("2"), Some(true));
        assert_eq!(convert_to_bool("-1"), Some(true));
        assert_eq!(convert_to_bool("0.0"), Some(false));
    }

    #[test]
    fn test_convert_to_bool_unknown() {
        assert_eq!(convert_to_bool("maybe"), None);
        assert_eq!(convert_to_bool("unknown"), None);
    }

    #[test]
    fn test_normalize_key_basic() {
        assert_eq!(normalize_key("Hello World"), "hello_world");
        assert_eq!(normalize_key("foo-bar"), "foo_bar");
        assert_eq!(normalize_key("FOO BAR"), "foo_bar");
        assert_eq!(normalize_key("foo__bar"), "foo_bar");
        assert_eq!(normalize_key("  foo  "), "foo");
    }

    #[test]
    fn test_normalize_key_special_chars() {
        assert_eq!(normalize_key("foo.bar"), "foo_bar");
        assert_eq!(normalize_key("foo/bar"), "foo_bar");
        assert_eq!(normalize_key("foo(bar)"), "foo_bar");
        assert_eq!(normalize_key("_foo"), "_foo");
    }

    #[test]
    fn test_remove_quotes() {
        assert_eq!(remove_quotes(r#""hello""#), "hello");
        assert_eq!(remove_quotes("'world'"), "world");
        assert_eq!(remove_quotes("plain"), "plain");
        assert_eq!(remove_quotes(r#""mixed'"#), r#""mixed'"#);
    }

    #[test]
    fn test_convert_size_to_int() {
        assert_eq!(convert_size_to_int("42", false), Some(42));
        assert_eq!(convert_size_to_int("1 KB", false), Some(1000));
        assert_eq!(convert_size_to_int("1 KiB", false), Some(1024));
        assert_eq!(convert_size_to_int("1 KB", true), Some(1024));
        assert_eq!(convert_size_to_int("1.5 GB", false), Some(1_500_000_000));
        assert_eq!(convert_size_to_int("5 bytes", false), Some(5));
    }

    #[test]
    fn test_has_data() {
        assert!(has_data("hello"));
        assert!(!has_data("   "));
        assert!(!has_data(""));
    }
}
