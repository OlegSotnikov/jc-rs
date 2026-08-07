//! Output formatting for jc-rs CLI.
//!
//! Handles JSON and YAML serialization, colorization (via the `colored` crate),
//! and broken-pipe error suppression.

use colored::Colorize;
use serde_json::Value;
use std::io::{self, IsTerminal, Write};

/// Color scheme for JSON output. Keys are determined by `JC_COLORS` env var.
#[derive(Debug, Clone)]
pub struct ColorScheme {
    /// Color name for object keys
    pub keys: String,
    /// Color name for booleans/null
    pub keywords: String,
    /// Color name for numbers
    pub numbers: String,
    /// Color name for strings
    pub strings: String,
}

impl Default for ColorScheme {
    fn default() -> Self {
        ColorScheme {
            keys: "blue".to_string(),
            keywords: "bright_black".to_string(),
            numbers: "magenta".to_string(),
            strings: "green".to_string(),
        }
    }
}

impl ColorScheme {
    /// Parse JC_COLORS environment variable.
    /// Format: `keyname_color,keyword_color,number_color,string_color`
    pub fn from_env() -> Self {
        let mut scheme = ColorScheme::default();
        if let Ok(env_val) = std::env::var("JC_COLORS") {
            let parts: Vec<&str> = env_val.split(',').collect();
            if parts.len() == 4 {
                let valid = |s: &str| {
                    matches!(
                        s,
                        "black"
                            | "red"
                            | "green"
                            | "yellow"
                            | "blue"
                            | "magenta"
                            | "cyan"
                            | "gray"
                            | "brightblack"
                            | "brightred"
                            | "brightgreen"
                            | "brightyellow"
                            | "brightblue"
                            | "brightmagenta"
                            | "brightcyan"
                            | "white"
                            | "default"
                    )
                };
                if parts.iter().all(|p| valid(p)) {
                    if parts[0] != "default" {
                        scheme.keys = normalize_color(parts[0]);
                    }
                    if parts[1] != "default" {
                        scheme.keywords = normalize_color(parts[1]);
                    }
                    if parts[2] != "default" {
                        scheme.numbers = normalize_color(parts[2]);
                    }
                    if parts[3] != "default" {
                        scheme.strings = normalize_color(parts[3]);
                    }
                } else {
                    eprintln!("jc-rs: warning - Could not parse JC_COLORS environment variable");
                }
            } else {
                eprintln!("jc-rs: warning - Could not parse JC_COLORS environment variable");
            }
        }
        scheme
    }
}

/// Normalize color name for `colored` crate (brightblack → bright_black).
fn normalize_color(s: &str) -> String {
    match s {
        "brightblack" => "bright_black".to_string(),
        "brightred" => "bright_red".to_string(),
        "brightgreen" => "bright_green".to_string(),
        "brightyellow" => "bright_yellow".to_string(),
        "brightblue" => "bright_blue".to_string(),
        "brightmagenta" => "bright_magenta".to_string(),
        "brightcyan" => "bright_cyan".to_string(),
        other => other.to_string(),
    }
}

/// Apply a named color to a string using the `colored` crate.
fn colorize(s: &str, color: &str, bold: bool) -> String {
    let cs = match color {
        "black" => s.black().to_string(),
        "red" => s.red().to_string(),
        "green" => s.green().to_string(),
        "yellow" => s.yellow().to_string(),
        "blue" => s.blue().to_string(),
        "magenta" => s.magenta().to_string(),
        "cyan" => s.cyan().to_string(),
        "white" => s.white().to_string(),
        "bright_black" | "gray" => s.bright_black().to_string(),
        "bright_red" => s.bright_red().to_string(),
        "bright_green" => s.bright_green().to_string(),
        "bright_yellow" => s.bright_yellow().to_string(),
        "bright_blue" => s.bright_blue().to_string(),
        "bright_magenta" => s.bright_magenta().to_string(),
        "bright_cyan" => s.bright_cyan().to_string(),
        _ => s.to_string(),
    };
    if bold { cs.bold().to_string() } else { cs }
}

/// Colorize a JSON string token by token using a simple hand-rolled approach.
///
/// This avoids pulling in a full syntax highlighter and produces output
/// compatible with jc's defaults (keys=bold blue, strings=green,
/// numbers=magenta, booleans/null=bright_black).
pub fn colorize_json(json: &str, scheme: &ColorScheme) -> String {
    // We re-serialize token by token from the parsed Value to guarantee
    // well-formed output. The `json` parameter is the pre-formatted string.
    // We parse it back and re-emit with colors.
    match serde_json::from_str::<Value>(json) {
        Ok(v) => colorize_value(&v, scheme, 0, false),
        Err(_) => json.to_string(), // fallback: return as-is
    }
}

fn colorize_value(v: &Value, scheme: &ColorScheme, indent: usize, pretty: bool) -> String {
    // We build a pretty-printed colorized string by recursively processing the Value.
    match v {
        Value::Null => colorize("null", &scheme.keywords, false),
        Value::Bool(b) => colorize(if *b { "true" } else { "false" }, &scheme.keywords, false),
        Value::Number(n) => colorize(&n.to_string(), &scheme.numbers, false),
        Value::String(s) => {
            let escaped = serde_json::to_string(s).unwrap_or_else(|_| format!("{:?}", s));
            colorize(&escaped, &scheme.strings, false)
        }
        Value::Array(arr) => {
            if arr.is_empty() {
                return "[]".to_string();
            }
            if pretty {
                let inner_indent = indent + 2;
                let pad = " ".repeat(inner_indent);
                let close_pad = " ".repeat(indent);
                let items: Vec<String> = arr
                    .iter()
                    .map(|item| {
                        format!(
                            "{}{}",
                            pad,
                            colorize_value(item, scheme, inner_indent, pretty)
                        )
                    })
                    .collect();
                format!("[\n{}\n{}]", items.join(",\n"), close_pad)
            } else {
                let items: Vec<String> = arr
                    .iter()
                    .map(|item| colorize_value(item, scheme, 0, false))
                    .collect();
                format!("[{}]", items.join(","))
            }
        }
        Value::Object(map) => {
            if map.is_empty() {
                return "{}".to_string();
            }
            if pretty {
                let inner_indent = indent + 2;
                let pad = " ".repeat(inner_indent);
                let close_pad = " ".repeat(indent);
                let pairs: Vec<String> = map
                    .iter()
                    .map(|(k, val)| {
                        let key_str =
                            serde_json::to_string(k).unwrap_or_else(|_| format!("{:?}", k));
                        let colored_key = colorize(&key_str, &scheme.keys, true);
                        let colored_val = colorize_value(val, scheme, inner_indent, pretty);
                        format!("{}{}: {}", pad, colored_key, colored_val)
                    })
                    .collect();
                format!("{{\n{}\n{}}}", pairs.join(",\n"), close_pad)
            } else {
                let pairs: Vec<String> = map
                    .iter()
                    .map(|(k, val)| {
                        let key_str =
                            serde_json::to_string(k).unwrap_or_else(|_| format!("{:?}", k));
                        let colored_key = colorize(&key_str, &scheme.keys, true);
                        let colored_val = colorize_value(val, scheme, 0, false);
                        format!("{}:{}", colored_key, colored_val)
                    })
                    .collect();
                format!("{{{}}}", pairs.join(","))
            }
        }
    }
}

/// Determine if we should output color.
///
/// Color is enabled if:
/// - stdout is a TTY, OR force_color is set
/// - AND NO_COLOR env var is not set
/// - AND mono option is not set
pub fn should_use_color(force_color: bool, mono: bool) -> bool {
    if std::env::var("NO_COLOR").is_ok() && !force_color {
        return false;
    }
    if mono && !force_color {
        return false;
    }
    if !io::stdout().is_terminal() && !force_color {
        return false;
    }
    true
}

/// Serialize a serde_json::Value to a JSON string.
pub fn to_json_string(value: &Value, pretty: bool) -> String {
    if pretty {
        serde_json::to_string_pretty(value).unwrap_or_else(|e| format!("{{\"error\":\"{}\"}}", e))
    } else {
        serde_json::to_string(value).unwrap_or_else(|e| format!("{{\"error\":\"{}\"}}", e))
    }
}

/// Print output to a writer, handling BrokenPipe gracefully.
/// (Used in tests to capture output without touching stdout.)
#[cfg(test)]
pub fn write_output_to_string(
    value: &Value,
    pretty: bool,
    yaml: bool,
    use_color: bool,
    scheme: &ColorScheme,
) -> String {
    if yaml {
        match serde_yaml::to_string(value) {
            Ok(s) => s.trim_end().to_string(),
            Err(_) => to_json_string(value, pretty),
        }
    } else {
        let json_str = to_json_string(value, pretty);
        if use_color {
            colorize_json(&json_str, scheme)
        } else {
            json_str
        }
    }
}

/// Print output to stdout, handling BrokenPipe gracefully.
pub fn print_output(
    value: &Value,
    pretty: bool,
    yaml: bool,
    use_color: bool,
    scheme: &ColorScheme,
    unbuffer: bool,
) {
    let text = render_output(value, pretty, yaml, use_color, scheme);

    let stdout = io::stdout();
    let mut handle = stdout.lock();
    let write_result = if unbuffer {
        writeln!(handle, "{}", text).and_then(|_| handle.flush())
    } else {
        writeln!(handle, "{}", text)
    };

    if let Err(e) = write_result {
        if e.kind() == io::ErrorKind::BrokenPipe {
            // Ignore broken pipe; it is normal when piping to head/less
        } else {
            eprintln!("jc-rs: error writing output: {}", e);
        }
    }
}

/// Serialize one value exactly as `print_output` would, without writing it.
///
/// The streaming path needs the text rather than the side effect: it owns a
/// buffered writer so that a long run does not pay a syscall per record.
pub fn render_output(
    value: &Value,
    pretty: bool,
    yaml: bool,
    use_color: bool,
    scheme: &ColorScheme,
) -> String {
    if yaml {
        match serde_yaml::to_string(value) {
            Ok(s) => {
                // serde_yaml adds a leading "---\n"; strip it to match jc behavior
                // Actually keep it: jc uses ruamel which adds "---"
                s.trim_end().to_string()
            }
            Err(e) => {
                eprintln!(
                    "jc-rs: warning - YAML serialization failed: {}. Falling back to JSON.",
                    e
                );
                to_json_string(value, pretty)
            }
        }
    } else {
        let json_str = to_json_string(value, pretty);
        if use_color {
            colorize_json(&json_str, scheme)
        } else {
            json_str
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::sync::Mutex;

    // Serialize env-var tests to avoid races between parallel test threads.
    static ENV_MUTEX: Mutex<()> = Mutex::new(());

    // ── ColorScheme::default ──────────────────────────────────────────────────

    #[test]
    fn test_color_scheme_default_fields() {
        let s = ColorScheme::default();
        assert_eq!(s.keys, "blue");
        assert_eq!(s.keywords, "bright_black");
        assert_eq!(s.numbers, "magenta");
        assert_eq!(s.strings, "green");
    }

    // ── ColorScheme::from_env ─────────────────────────────────────────────────

    #[test]
    fn test_from_env_valid_four_part() {
        let _guard = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        unsafe {
            std::env::set_var("JC_COLORS", "green,red,blue,cyan");
        }
        let scheme = ColorScheme::from_env();
        unsafe {
            std::env::remove_var("JC_COLORS");
        }
        assert_eq!(scheme.keys, "green");
        assert_eq!(scheme.keywords, "red");
        assert_eq!(scheme.numbers, "blue");
        assert_eq!(scheme.strings, "cyan");
    }

    #[test]
    fn test_from_env_valid_with_brightcolors() {
        let _guard = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        unsafe {
            std::env::set_var("JC_COLORS", "brightblack,brightred,brightgreen,brightcyan");
        }
        let scheme = ColorScheme::from_env();
        unsafe {
            std::env::remove_var("JC_COLORS");
        }
        assert_eq!(scheme.keys, "bright_black");
        assert_eq!(scheme.keywords, "bright_red");
        assert_eq!(scheme.numbers, "bright_green");
        assert_eq!(scheme.strings, "bright_cyan");
    }

    #[test]
    fn test_from_env_default_keyword_keeps_default() {
        let _guard = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        // "default" means keep the scheme default for that slot
        unsafe {
            std::env::set_var("JC_COLORS", "default,default,default,default");
        }
        let scheme = ColorScheme::from_env();
        unsafe {
            std::env::remove_var("JC_COLORS");
        }
        let def = ColorScheme::default();
        assert_eq!(scheme.keys, def.keys);
        assert_eq!(scheme.numbers, def.numbers);
    }

    #[test]
    fn test_from_env_too_few_parts_falls_back_to_default() {
        let _guard = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        unsafe {
            std::env::set_var("JC_COLORS", "green,red,blue");
        } // only 3
        let scheme = ColorScheme::from_env();
        unsafe {
            std::env::remove_var("JC_COLORS");
        }
        let def = ColorScheme::default();
        assert_eq!(scheme.keys, def.keys);
        assert_eq!(scheme.strings, def.strings);
    }

    #[test]
    fn test_from_env_invalid_color_name_falls_back_to_default() {
        let _guard = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        unsafe {
            std::env::set_var("JC_COLORS", "notacolor,red,blue,cyan");
        }
        let scheme = ColorScheme::from_env();
        unsafe {
            std::env::remove_var("JC_COLORS");
        }
        let def = ColorScheme::default();
        // All slots fall back to defaults when any color is invalid
        assert_eq!(scheme.keys, def.keys);
    }

    #[test]
    fn test_from_env_missing_var_falls_back_to_default() {
        let _guard = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        unsafe {
            std::env::remove_var("JC_COLORS");
        }
        let scheme = ColorScheme::from_env();
        let def = ColorScheme::default();
        assert_eq!(scheme.keys, def.keys);
        assert_eq!(scheme.strings, def.strings);
    }

    // ── normalize_color ───────────────────────────────────────────────────────

    #[test]
    fn test_normalize_color_bright_variants() {
        assert_eq!(normalize_color("brightblack"), "bright_black");
        assert_eq!(normalize_color("brightred"), "bright_red");
        assert_eq!(normalize_color("brightgreen"), "bright_green");
        assert_eq!(normalize_color("brightyellow"), "bright_yellow");
        assert_eq!(normalize_color("brightblue"), "bright_blue");
        assert_eq!(normalize_color("brightmagenta"), "bright_magenta");
        assert_eq!(normalize_color("brightcyan"), "bright_cyan");
    }

    #[test]
    fn test_normalize_color_passthrough() {
        assert_eq!(normalize_color("red"), "red");
        assert_eq!(normalize_color("blue"), "blue");
        assert_eq!(normalize_color("green"), "green");
        assert_eq!(normalize_color("unknown"), "unknown");
    }

    // ── to_json_string ────────────────────────────────────────────────────────

    #[test]
    fn test_format_json_compact() {
        let v = json!({"key": "value", "n": 42});
        let s = to_json_string(&v, false);
        // Compact: no newlines, no extra spaces around colons
        assert!(!s.contains('\n'));
        assert!(s.contains("\"key\""));
        assert!(s.contains("42"));
    }

    #[test]
    fn test_format_json_pretty() {
        let v = json!({"key": "value", "n": 42});
        let s = to_json_string(&v, true);
        // Pretty-printed: has newlines and indentation
        assert!(s.contains('\n'));
        assert!(s.contains("  "));
    }

    #[test]
    fn test_format_json_array_compact() {
        let v = json!([1, 2, 3]);
        let s = to_json_string(&v, false);
        assert_eq!(s, "[1,2,3]");
    }

    #[test]
    fn test_format_json_array_pretty() {
        let v = json!([1, 2, 3]);
        let s = to_json_string(&v, true);
        assert!(s.contains('\n'));
        assert!(s.starts_with('['));
        assert!(s.ends_with(']'));
    }

    // ── YAML output ───────────────────────────────────────────────────────────

    #[test]
    fn test_yaml_output_round_trip() {
        let v = json!({"name": "alice", "age": 30});
        let yaml_str = serde_yaml::to_string(&v).expect("serialize");
        // Parse back
        let back: serde_json::Value = serde_yaml::from_str(&yaml_str).expect("deserialize");
        assert_eq!(back["name"], json!("alice"));
        assert_eq!(back["age"], json!(30));
    }

    #[test]
    fn test_yaml_output_contains_keys() {
        let scheme = ColorScheme::default();
        let v = json!({"status": "ok", "count": 5});
        let result = write_output_to_string(&v, false, true, false, &scheme);
        assert!(result.contains("status"));
        assert!(result.contains("ok"));
        assert!(result.contains("count"));
    }

    // ── should_use_color ─────────────────────────────────────────────────────

    #[test]
    fn test_should_use_color_mono_returns_false() {
        // mono=true without force_color → always false
        assert!(!should_use_color(false, true));
    }

    #[test]
    fn test_should_use_color_force_color_overrides_mono() {
        // force_color=true should override mono=true
        assert!(should_use_color(true, true));
    }

    #[test]
    fn test_should_use_color_no_color_env() {
        let _guard = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        unsafe {
            std::env::set_var("NO_COLOR", "1");
        }
        let result = should_use_color(false, false);
        unsafe {
            std::env::remove_var("NO_COLOR");
        }
        assert!(!result);
    }

    #[test]
    fn test_should_use_color_force_overrides_no_color_env() {
        let _guard = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        unsafe {
            std::env::set_var("NO_COLOR", "1");
        }
        let result = should_use_color(true, false);
        unsafe {
            std::env::remove_var("NO_COLOR");
        }
        assert!(result);
    }
}
