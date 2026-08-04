//! Parser for `xrandr` command output.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use regex::Regex;
use serde_json::{Map, Value};
use std::sync::OnceLock;

pub struct XrandrParser;

static INFO: ParserInfo = ParserInfo {
    name: "xrandr",
    argument: "--xrandr",
    version: "2.1.0",
    description: "Converts `xrandr` command output to JSON",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[Platform::Linux, Platform::Darwin, Platform::FreeBSD],
    tags: &[Tag::Command],
    magic_commands: &["xrandr"],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static XRANDR_PARSER: XrandrParser = XrandrParser;

inventory::submit! {
    ParserEntry::new(&XRANDR_PARSER)
}

static SCREEN_RE: OnceLock<Regex> = OnceLock::new();
static DEVICE_RE: OnceLock<Regex> = OnceLock::new();
static RESOLUTION_RE: OnceLock<Regex> = OnceLock::new();
static FREQ_RE: OnceLock<Regex> = OnceLock::new();
static PROP_KEY_RE: OnceLock<Regex> = OnceLock::new();
static IGNORE_RE: OnceLock<Regex> = OnceLock::new();

fn get_screen_re() -> &'static Regex {
    SCREEN_RE.get_or_init(|| {
        Regex::new(
            r"Screen (\d+): minimum (\d+) x (\d+), current (\d+) x (\d+), maximum (\d+) x (\d+)",
        )
        .unwrap()
    })
}

fn get_device_re() -> &'static Regex {
    DEVICE_RE.get_or_init(|| {
        Regex::new(
            r"^(\S+)\s+(connected|disconnected)(\s+primary)?(?:\s+(\d+)x(\d+)\+(\d+)\+(\d+))?\s*(normal|right|left|inverted)?(?:\s*(X axis|Y axis|X and Y axis))?(?:\s*\(normal left inverted right x axis y axis\))?(?:\s*(\d+)mm x (\d+)mm)?",
        )
        .unwrap()
    })
}

fn get_resolution_re() -> &'static Regex {
    RESOLUTION_RE.get_or_init(|| Regex::new(r"^\s+(\d+)x(\d+)(i)?\s+(.*)").unwrap())
}

fn get_freq_re() -> &'static Regex {
    FREQ_RE.get_or_init(|| {
        // Match: freq [*|space] [+]
        // star can be '*' (is_current=true) or ' ' (is_current=false, but preferred may follow)
        Regex::new(r"(\d+\.\d+)([* ])?(\+?)").unwrap()
    })
}

fn get_prop_key_re() -> &'static Regex {
    PROP_KEY_RE.get_or_init(|| Regex::new(r"^\t([\w| |\-|_]+):\s?(.*)").unwrap())
}

fn get_ignore_re() -> &'static Regex {
    IGNORE_RE.get_or_init(|| {
        Regex::new(r"^\s+(h|v):\s+(height|width)\s+\d+\s+start\s+\d+\s+end").unwrap()
    })
}

/// Count leading spaces/tabs to determine indentation level.
fn indent_level(line: &str) -> usize {
    let mut count = 0;
    for ch in line.chars() {
        match ch {
            ' ' => count += 1,
            '\t' => count += 4, // treat tab as 4 spaces for level detection
            _ => break,
        }
    }
    count
}

/// Parse frequency tokens from a resolution mode rest string.
fn parse_frequencies(rest: &str) -> Vec<Map<String, Value>> {
    let mut freqs = Vec::new();
    for caps in get_freq_re().captures_iter(rest) {
        let freq_str = &caps[1];
        // star: "*" means is_current=true, " " or absent means false
        let star = caps.get(2).map_or("", |m| m.as_str());
        let is_current = star == "*";
        let plus = caps.get(3).map_or("", |m| m.as_str());
        let is_preferred = plus == "+";

        if let Ok(f) = freq_str.parse::<f64>() {
            let mut freq_obj = Map::new();
            if let Some(n) = serde_json::Number::from_f64(f) {
                freq_obj.insert("frequency".to_string(), Value::Number(n));
            }
            freq_obj.insert("is_current".to_string(), Value::Bool(is_current));
            freq_obj.insert("is_preferred".to_string(), Value::Bool(is_preferred));
            freqs.push(freq_obj);
        }
    }
    freqs
}

impl Parser for XrandrParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            let mut obj = Map::new();
            obj.insert("screens".to_string(), Value::Array(vec![]));
            return Ok(ParseOutput::Object(obj));
        }

        let lines: Vec<&str> = input.lines().collect();
        let mut screens: Vec<Value> = Vec::new();

        // Current state
        let mut current_screen: Option<Map<String, Value>> = None;
        let mut current_device: Option<Map<String, Value>> = None;

        // Prop tracking
        let mut in_props = false;
        let mut prop_key: Option<String> = None;
        let mut prop_values: Vec<String> = Vec::new();
        let mut props_obj: Map<String, Value> = Map::new();

        let flush_prop = |prop_key: &mut Option<String>,
                          prop_values: &mut Vec<String>,
                          props_obj: &mut Map<String, Value>| {
            if let Some(key) = prop_key.take() {
                if prop_values.len() == 1 {
                    props_obj.insert(key, Value::String(prop_values[0].clone()));
                } else if !prop_values.is_empty() {
                    props_obj.insert(
                        key,
                        Value::Array(
                            prop_values
                                .iter()
                                .map(|s| Value::String(s.clone()))
                                .collect(),
                        ),
                    );
                }
                prop_values.clear();
            }
        };

        let flush_device = |current_device: &mut Option<Map<String, Value>>,
                            current_screen: &mut Option<Map<String, Value>>,
                            prop_key: &mut Option<String>,
                            prop_values: &mut Vec<String>,
                            props_obj: &mut Map<String, Value>,
                            in_props: &mut bool| {
            flush_prop(prop_key, prop_values, props_obj);
            if let Some(mut dev) = current_device.take() {
                if *in_props {
                    dev.insert("props".to_string(), Value::Object(props_obj.clone()));
                    props_obj.clear();
                    *in_props = false;
                } else if !dev.contains_key("props") {
                    dev.insert("props".to_string(), Value::Object(Map::new()));
                }
                if let Some(screen) = current_screen {
                    let devices = screen
                        .entry("devices".to_string())
                        .or_insert_with(|| Value::Array(vec![]));
                    if let Value::Array(arr) = devices {
                        arr.push(Value::Object(dev));
                    }
                }
            }
        };

        for &line in &lines {
            // Ignore h:/v: lines
            if get_ignore_re().is_match(line) {
                continue;
            }

            // Check indentation
            let indent = indent_level(line);
            let trimmed = line.trim();

            // Screen line (no indentation)
            if let Some(caps) = get_screen_re().captures(trimmed) {
                // Flush current device and screen
                flush_device(
                    &mut current_device,
                    &mut current_screen,
                    &mut prop_key,
                    &mut prop_values,
                    &mut props_obj,
                    &mut in_props,
                );
                if let Some(screen) = current_screen.take() {
                    screens.push(Value::Object(screen));
                }

                let mut screen = Map::new();
                let screen_num: i64 = caps[1].parse().unwrap_or(0);
                let min_w: i64 = caps[2].parse().unwrap_or(0);
                let min_h: i64 = caps[3].parse().unwrap_or(0);
                let cur_w: i64 = caps[4].parse().unwrap_or(0);
                let cur_h: i64 = caps[5].parse().unwrap_or(0);
                let max_w: i64 = caps[6].parse().unwrap_or(0);
                let max_h: i64 = caps[7].parse().unwrap_or(0);

                screen.insert("devices".to_string(), Value::Array(vec![]));
                screen.insert(
                    "screen_number".to_string(),
                    Value::Number(screen_num.into()),
                );
                screen.insert("minimum_width".to_string(), Value::Number(min_w.into()));
                screen.insert("minimum_height".to_string(), Value::Number(min_h.into()));
                screen.insert("current_width".to_string(), Value::Number(cur_w.into()));
                screen.insert("current_height".to_string(), Value::Number(cur_h.into()));
                screen.insert("maximum_width".to_string(), Value::Number(max_w.into()));
                screen.insert("maximum_height".to_string(), Value::Number(max_h.into()));
                current_screen = Some(screen);
                in_props = false;
                continue;
            }

            // Device line (no indentation, starts with device name)
            if indent == 0
                && !trimmed.is_empty()
                && let Some(caps) = get_device_re().captures(line)
            {
                // Flush current device
                flush_device(
                    &mut current_device,
                    &mut current_screen,
                    &mut prop_key,
                    &mut prop_values,
                    &mut props_obj,
                    &mut in_props,
                );
                in_props = false;

                let device_name = caps[1].to_string();
                let is_connected = &caps[2] == "connected";
                let is_primary = caps
                    .get(3)
                    .map(|m| !m.as_str().trim().is_empty())
                    .unwrap_or(false);

                let mut dev = Map::new();
                dev.insert("props".to_string(), Value::Object(Map::new()));
                dev.insert("resolution_modes".to_string(), Value::Array(vec![]));
                dev.insert("is_connected".to_string(), Value::Bool(is_connected));
                dev.insert("is_primary".to_string(), Value::Bool(is_primary));
                dev.insert("device_name".to_string(), Value::String(device_name));

                // rotation (default "normal")
                let rotation = caps
                    .get(8)
                    .map(|m| m.as_str().to_string())
                    .filter(|s| !s.is_empty())
                    .unwrap_or_else(|| "normal".to_string());
                dev.insert("rotation".to_string(), Value::String(rotation));

                // reflection (default "normal")
                let reflection = caps
                    .get(9)
                    .map(|m| m.as_str().to_string())
                    .filter(|s| !s.is_empty())
                    .unwrap_or_else(|| "normal".to_string());
                dev.insert("reflection".to_string(), Value::String(reflection));

                // Optional resolution+offset
                if let (Some(rw), Some(rh), Some(ow), Some(oh)) =
                    (caps.get(4), caps.get(5), caps.get(6), caps.get(7))
                    && let (Ok(rw), Ok(rh), Ok(ow), Ok(oh)) = (
                        rw.as_str().parse::<i64>(),
                        rh.as_str().parse::<i64>(),
                        ow.as_str().parse::<i64>(),
                        oh.as_str().parse::<i64>(),
                    )
                {
                    dev.insert("resolution_width".to_string(), Value::Number(rw.into()));
                    dev.insert("resolution_height".to_string(), Value::Number(rh.into()));
                    dev.insert("offset_width".to_string(), Value::Number(ow.into()));
                    dev.insert("offset_height".to_string(), Value::Number(oh.into()));
                }

                // Optional dimensions
                if let (Some(dw), Some(dh)) = (caps.get(10), caps.get(11))
                    && let (Ok(dw), Ok(dh)) =
                        (dw.as_str().parse::<i64>(), dh.as_str().parse::<i64>())
                {
                    dev.insert("dimension_width".to_string(), Value::Number(dw.into()));
                    dev.insert("dimension_height".to_string(), Value::Number(dh.into()));
                }

                current_device = Some(dev);
                continue;
            }

            // Resolution mode line (starts with spaces, has WxH format)
            if indent > 0
                && indent <= 4
                && let Some(caps) = get_resolution_re().captures(line)
            {
                let rw: i64 = caps[1].parse().unwrap_or(0);
                let rh: i64 = caps[2].parse().unwrap_or(0);
                let is_high_res = caps.get(3).is_some_and(|m| !m.as_str().is_empty());
                let rest = &caps[4];

                let freqs = parse_frequencies(rest);

                let mut mode = Map::new();
                mode.insert("resolution_width".to_string(), Value::Number(rw.into()));
                mode.insert("resolution_height".to_string(), Value::Number(rh.into()));
                mode.insert("is_high_resolution".to_string(), Value::Bool(is_high_res));
                mode.insert(
                    "frequencies".to_string(),
                    Value::Array(freqs.into_iter().map(Value::Object).collect()),
                );

                if let Some(ref mut dev) = current_device {
                    let modes = dev
                        .entry("resolution_modes".to_string())
                        .or_insert_with(|| Value::Array(vec![]));
                    if let Value::Array(arr) = modes {
                        arr.push(Value::Object(mode));
                    }
                }
                in_props = false;
                continue;
            }

            // Property key line (single tab = 4-8 spaces indent)
            if (4..8).contains(&indent)
                && let Some(caps) = get_prop_key_re().captures(line)
            {
                flush_prop(&mut prop_key, &mut prop_values, &mut props_obj);
                in_props = true;
                let key = caps[1].trim().to_string();
                let maybe_val = caps[2].trim().to_string();
                prop_key = Some(key);
                if !maybe_val.is_empty() {
                    prop_values.push(maybe_val);
                }
                continue;
            }

            // Property value line (double tab = 8+ spaces)
            if indent >= 8 && in_props {
                prop_values.push(trimmed.to_string());
                continue;
            }
        }

        // Flush last device and screen
        flush_device(
            &mut current_device,
            &mut current_screen,
            &mut prop_key,
            &mut prop_values,
            &mut props_obj,
            &mut in_props,
        );
        if let Some(screen) = current_screen.take() {
            screens.push(Value::Object(screen));
        }

        let mut result = Map::new();
        result.insert("screens".to_string(), Value::Array(screens));
        Ok(ParseOutput::Object(result))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check_xrandr(input: &str, expected_json: &str) {
        let parser = XrandrParser;
        let result = parser.parse(input, false).unwrap();
        let expected: serde_json::Value = serde_json::from_str(expected_json).unwrap();
        match result {
            ParseOutput::Object(obj) => {
                let got_screens = obj.get("screens").and_then(|v| v.as_array()).unwrap();
                let exp_screens = expected.get("screens").and_then(|v| v.as_array()).unwrap();
                assert_eq!(
                    got_screens.len(),
                    exp_screens.len(),
                    "screen count mismatch"
                );

                for (si, (gs, es)) in got_screens.iter().zip(exp_screens.iter()).enumerate() {
                    // Screen-level fields
                    for field in &[
                        "screen_number",
                        "minimum_width",
                        "minimum_height",
                        "current_width",
                        "current_height",
                        "maximum_width",
                        "maximum_height",
                    ] {
                        assert_eq!(
                            gs.get(field).unwrap_or(&Value::Null),
                            es.get(field).unwrap_or(&Value::Null),
                            "screen {} field '{}' mismatch",
                            si,
                            field
                        );
                    }

                    let got_devs = gs.get("devices").and_then(|v| v.as_array()).unwrap();
                    let exp_devs = es.get("devices").and_then(|v| v.as_array()).unwrap();
                    assert_eq!(
                        got_devs.len(),
                        exp_devs.len(),
                        "screen {} device count mismatch",
                        si
                    );

                    for (di, (gd, ed)) in got_devs.iter().zip(exp_devs.iter()).enumerate() {
                        for field in &[
                            "device_name",
                            "is_connected",
                            "is_primary",
                            "rotation",
                            "reflection",
                        ] {
                            assert_eq!(
                                gd.get(field).unwrap_or(&Value::Null),
                                ed.get(field).unwrap_or(&Value::Null),
                                "screen {} device {} field '{}' mismatch",
                                si,
                                di,
                                field
                            );
                        }

                        // Resolution fields (optional)
                        for field in &[
                            "resolution_width",
                            "resolution_height",
                            "offset_width",
                            "offset_height",
                            "dimension_width",
                            "dimension_height",
                        ] {
                            let ev = ed.get(field);
                            if ev.is_some() && ev != Some(&Value::Null) {
                                assert_eq!(
                                    gd.get(field).unwrap_or(&Value::Null),
                                    ev.unwrap_or(&Value::Null),
                                    "screen {} device {} field '{}' mismatch",
                                    si,
                                    di,
                                    field
                                );
                            }
                        }

                        // Check resolution mode count
                        let got_modes = gd.get("resolution_modes").and_then(|v| v.as_array());
                        let exp_modes = ed.get("resolution_modes").and_then(|v| v.as_array());
                        if let (Some(gm), Some(em)) = (got_modes, exp_modes) {
                            assert_eq!(
                                gm.len(),
                                em.len(),
                                "screen {} device {} mode count mismatch",
                                si,
                                di
                            );

                            for (mi, (gmode, emode)) in gm.iter().zip(em.iter()).enumerate() {
                                for field in &[
                                    "resolution_width",
                                    "resolution_height",
                                    "is_high_resolution",
                                ] {
                                    assert_eq!(
                                        gmode.get(field).unwrap_or(&Value::Null),
                                        emode.get(field).unwrap_or(&Value::Null),
                                        "screen {} device {} mode {} field '{}' mismatch",
                                        si,
                                        di,
                                        mi,
                                        field
                                    );
                                }
                                // Check frequency count
                                let gfreqs = gmode.get("frequencies").and_then(|v| v.as_array());
                                let efreqs = emode.get("frequencies").and_then(|v| v.as_array());
                                if let (Some(gf), Some(ef)) = (gfreqs, efreqs) {
                                    assert_eq!(
                                        gf.len(),
                                        ef.len(),
                                        "screen {} device {} mode {} freq count mismatch",
                                        si,
                                        di,
                                        mi
                                    );
                                    for (fi, (gfreq, efreq)) in gf.iter().zip(ef.iter()).enumerate()
                                    {
                                        for field in &["frequency", "is_current", "is_preferred"] {
                                            assert_eq!(
                                                gfreq.get(field).unwrap_or(&Value::Null),
                                                efreq.get(field).unwrap_or(&Value::Null),
                                                "screen {} device {} mode {} freq {} field '{}' mismatch",
                                                si,
                                                di,
                                                mi,
                                                fi,
                                                field
                                            );
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            _ => panic!("expected Object"),
        }
    }

    #[test]
    fn test_xrandr_simple() {
        check_xrandr(
            include_str!("../../../../tests/fixtures/generic/xrandr_simple.out"),
            include_str!("../../../../tests/fixtures/generic/xrandr_simple.json"),
        );
    }

    #[test]
    fn test_xrandr_main() {
        check_xrandr(
            include_str!("../../../../tests/fixtures/generic/xrandr.out"),
            include_str!("../../../../tests/fixtures/generic/xrandr.json"),
        );
    }

    #[test]
    fn test_xrandr_3() {
        check_xrandr(
            include_str!("../../../../tests/fixtures/generic/xrandr_3.out"),
            include_str!("../../../../tests/fixtures/generic/xrandr_3.json"),
        );
    }

    #[test]
    fn test_xrandr_2() {
        check_xrandr(
            include_str!("../../../../tests/fixtures/generic/xrandr_2.out"),
            include_str!("../../../../tests/fixtures/generic/xrandr_2.json"),
        );
    }

    #[test]
    fn test_xrandr_extra_hv() {
        check_xrandr(
            include_str!("../../../../tests/fixtures/generic/xrandr_extra_hv_lines.out"),
            include_str!("../../../../tests/fixtures/generic/xrandr_extra_hv_lines.json"),
        );
    }

    #[test]
    fn test_xrandr_empty() {
        let parser = XrandrParser;
        let result = parser.parse("", false).unwrap();
        match result {
            ParseOutput::Object(obj) => {
                let screens = obj.get("screens").and_then(|v| v.as_array()).unwrap();
                assert!(screens.is_empty());
            }
            _ => panic!("expected Object"),
        }
    }
}
