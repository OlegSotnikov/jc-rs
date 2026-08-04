//! Conversion utilities ported from jc's utils.py.

use regex::Regex;
use std::sync::OnceLock;

fn non_numeric_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"[^0-9\-\.]").unwrap())
}

/// Convert a string to i64 by stripping non-numeric characters.
///
/// Matches jc's `convert_to_int`: strips everything except digits, `-`, `.`,
/// tries int parse first, then float-to-int.
pub fn convert_to_int(value: &str) -> Option<i64> {
    let cleaned = non_numeric_re().replace_all(value, "");
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
    let cleaned = non_numeric_re().replace_all(value, "");
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
    // Non-numeric string
    if value.is_empty() {
        return Some(false);
    }
    let lower = value.to_lowercase();
    match lower.as_str() {
        "y" | "yes" | "true" | "*" => Some(true),
        "n" | "no" | "false" | "0" => Some(false),
        _ => None,
    }
}

/// Parse a human-readable size string like "10KB", "5.2 MiB" into bytes.
///
/// `binary_mode`: treat ambiguous units (KB, MB, etc.) as binary (1024-based).
///
/// Mirrors jc's `convert_size_to_int` with `binary` parameter.
pub fn convert_size_to_int(size: &str, binary_mode: bool) -> Option<i64> {
    // Remove commas
    let size = size.replace(',', "");
    let size = size.trim();

    // Tokenize: split on digit sequences
    let token_re = Regex::new(r"(\d+(?:\.\d+)?)").ok()?;
    let mut tokens: Vec<String> = Vec::new();
    let mut last_end = 0;
    for m in token_re.find_iter(size) {
        let before = size[last_end..m.start()].trim();
        if !before.is_empty() {
            tokens.push(before.to_string());
        }
        tokens.push(m.as_str().to_string());
        last_end = m.end();
    }
    let remainder = size[last_end..].trim();
    if !remainder.is_empty() {
        tokens.push(remainder.to_string());
    }

    if tokens.is_empty() {
        return None;
    }

    // First token must be a number
    let num_str = &tokens[0];
    let num: f64 = num_str.parse().ok()?;

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
    // Same set as jc: !"#$%&'()*+,-./:;<=>?@[\]^`{|}~ and space
    const SPECIAL: &str = "!\"#$%&'()*+,-./:;<=>?@[\\]^`{|}~ ";
    let mut data = key.trim().to_lowercase();

    for ch in SPECIAL.chars() {
        data = data.replace(ch, "_");
    }

    let initial_underscore = data.starts_with('_');

    // Strip leading/trailing underscores, split on underscore (compresses multiples), rejoin
    let stripped = data.trim_matches('_');
    let parts: Vec<&str> = stripped.split('_').filter(|s| !s.is_empty()).collect();
    let mut result = parts.join("_");

    if initial_underscore {
        result = format!("_{}", result);
    }

    result
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
