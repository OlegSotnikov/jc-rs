//! Parser for `update-alternatives --query` command output.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};

pub struct UpdateAltQParser;

static INFO: ParserInfo = ParserInfo {
    name: "update_alt_q",
    argument: "--update-alt-q",
    version: "1.2.0",
    description: "Converts `update-alternatives --query` command output to JSON",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[Platform::Linux],
    tags: &[Tag::Command],
    magic_commands: &["update-alternatives --query"],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static UPDATE_ALT_Q_PARSER: UpdateAltQParser = UpdateAltQParser;

inventory::submit! {
    ParserEntry::new(&UPDATE_ALT_Q_PARSER)
}

impl Parser for UpdateAltQParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        let out = parse_update_alt_q(input);
        Ok(ParseOutput::Object(out))
    }
}

fn parse_update_alt_q(input: &str) -> Map<String, Value> {
    let mut out = Map::new();
    let mut top_slaves: Vec<Value> = Vec::new();
    let mut alternatives: Vec<Map<String, Value>> = Vec::new();
    let mut current_alt: Option<Map<String, Value>> = None;
    let mut current_alt_slaves: Vec<Value> = Vec::new();

    for line in input.lines() {
        if line.trim().is_empty() {
            continue;
        }

        // Split on first space: key and value
        let (key_part, val_part) = match line.split_once(' ') {
            Some((k, v)) => (k.trim_end_matches(':'), v.trim()),
            None => {
                // Line with no space - could be "Slaves:" header
                let stripped = line.trim().trim_end_matches(':');
                if stripped == "Slaves" {
                    // Next lines are slave entries (indented)
                }
                continue;
            }
        };

        if line.starts_with("Name: ") {
            out.insert("name".to_string(), Value::String(val_part.to_string()));
        } else if line.starts_with("Link: ") {
            out.insert("link".to_string(), Value::String(val_part.to_string()));
        } else if line.starts_with("Slaves:") {
            // Just a header - skip
        } else if line.starts_with(' ') {
            // Slave entry (indented): "  slave-name /path/to/slave"
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let s_name = parts[0].to_string();
                let s_path = parts[1..].join(" ");
                let mut slave = Map::new();
                slave.insert("name".to_string(), Value::String(s_name));
                slave.insert("path".to_string(), Value::String(s_path));
                let slave_val = Value::Object(slave);

                if current_alt.is_some() {
                    current_alt_slaves.push(slave_val);
                } else {
                    top_slaves.push(slave_val);
                }
            }
        } else if line.starts_with("Status: ") {
            // Before status, flush top-level slaves
            if !top_slaves.is_empty() {
                out.insert("slaves".to_string(), Value::Array(top_slaves.clone()));
                top_slaves.clear();
            }
            out.insert("status".to_string(), Value::String(val_part.to_string()));
        } else if line.starts_with("Best: ") {
            out.insert("best".to_string(), Value::String(val_part.to_string()));
        } else if line.starts_with("Value: ") {
            let v = if val_part == "none" {
                Value::Null
            } else {
                Value::String(val_part.to_string())
            };
            out.insert("value".to_string(), v);
        } else if line.starts_with("Alternative: ") {
            // Flush previous alternative
            if let Some(mut alt) = current_alt.take() {
                if !current_alt_slaves.is_empty() {
                    alt.insert(
                        "slaves".to_string(),
                        Value::Array(current_alt_slaves.clone()),
                    );
                    current_alt_slaves.clear();
                }
                alternatives.push(alt);
            }
            let mut alt = Map::new();
            alt.insert(
                "alternative".to_string(),
                Value::String(val_part.to_string()),
            );
            current_alt = Some(alt);
        } else if line.starts_with("Priority: ") {
            if let Some(ref mut alt) = current_alt {
                if let Ok(n) = val_part.parse::<i64>() {
                    alt.insert("priority".to_string(), Value::Number(n.into()));
                } else {
                    alt.insert("priority".to_string(), Value::String(val_part.to_string()));
                }
            }
        } else {
            // Generic key: value
            let _ = key_part;
        }
    }

    // Flush last alternative
    if let Some(mut alt) = current_alt {
        if !current_alt_slaves.is_empty() {
            alt.insert("slaves".to_string(), Value::Array(current_alt_slaves));
        }
        alternatives.push(alt);
    }

    // Flush top-level slaves if not yet flushed
    if !top_slaves.is_empty() {
        out.insert("slaves".to_string(), Value::Array(top_slaves));
    }

    if !alternatives.is_empty() {
        out.insert(
            "alternatives".to_string(),
            Value::Array(alternatives.into_iter().map(Value::Object).collect()),
        );
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_update_alt_q_basic() {
        let input = "Name: editor\n\
                     Link: /usr/bin/editor\n\
                     Slaves:\n\
                      editor.1.gz /usr/share/man/man1/editor.1.gz\n\
                     Status: auto\n\
                     Best: /bin/nano\n\
                     Value: /bin/nano\n\
                     \n\
                     Alternative: /bin/ed\n\
                     Priority: -100\n\
                      editor.1.gz /usr/share/man/man1/ed.1.gz\n\
                     \n\
                     Alternative: /bin/nano\n\
                     Priority: 40\n\
                      editor.1.gz /usr/share/man/man1/nano.1.gz";

        let parser = UpdateAltQParser;
        let result = parser.parse(input, false).unwrap();
        if let ParseOutput::Object(obj) = result {
            assert_eq!(obj.get("name"), Some(&Value::String("editor".to_string())));
            assert_eq!(
                obj.get("link"),
                Some(&Value::String("/usr/bin/editor".to_string()))
            );
            assert_eq!(obj.get("status"), Some(&Value::String("auto".to_string())));
            assert_eq!(
                obj.get("best"),
                Some(&Value::String("/bin/nano".to_string()))
            );
            assert_eq!(
                obj.get("value"),
                Some(&Value::String("/bin/nano".to_string()))
            );
            if let Some(Value::Array(alts)) = obj.get("alternatives") {
                assert_eq!(alts.len(), 2);
                if let Some(Value::Object(alt)) = alts.get(0) {
                    assert_eq!(
                        alt.get("alternative"),
                        Some(&Value::String("/bin/ed".to_string()))
                    );
                    assert_eq!(alt.get("priority"), Some(&Value::Number((-100_i64).into())));
                }
            } else {
                panic!("Expected alternatives array");
            }
        } else {
            panic!("Expected Object");
        }
    }

    #[test]
    fn test_update_alt_q_none_value() {
        let input = "Name: editor\n\
                     Link: /usr/bin/editor\n\
                     Status: manual\n\
                     Best: /bin/nano\n\
                     Value: none";

        let parser = UpdateAltQParser;
        let result = parser.parse(input, false).unwrap();
        if let ParseOutput::Object(obj) = result {
            assert_eq!(obj.get("value"), Some(&Value::Null));
        } else {
            panic!("Expected Object");
        }
    }

    #[test]
    fn test_update_alt_q_empty() {
        let parser = UpdateAltQParser;
        let result = parser.parse("", false).unwrap();
        if let ParseOutput::Object(obj) = result {
            assert!(obj.is_empty());
        } else {
            panic!("Expected Object");
        }
    }
}
