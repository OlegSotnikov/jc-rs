//! Parser for `needrestart -b` command output.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};

pub struct NeedrestartParser;

static INFO: ParserInfo = ParserInfo {
    name: "needrestart",
    argument: "--needrestart",
    version: "1.0.0",
    description: "Converts `needrestart -b` command output to JSON",
    author: "jc-rs contributors",
    author_email: "os@g1sw.com",
    compatible: &[Platform::Linux],
    tags: &[Tag::Command],
    magic_commands: &["needrestart -b"],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static NEEDRESTART_PARSER: NeedrestartParser = NeedrestartParser;

inventory::submit! {
    ParserEntry::new(&NEEDRESTART_PARSER)
}

impl Parser for NeedrestartParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        let out = parse_needrestart(input);
        Ok(ParseOutput::Object(out))
    }
}

fn normalize_key(key: &str) -> String {
    key.to_lowercase().replace('-', "_")
}

fn parse_needrestart(input: &str) -> Map<String, Value> {
    let mut out = Map::new();
    let mut sess_list: Vec<Value> = Vec::new();
    let mut svc_list: Vec<Value> = Vec::new();
    let mut pid_list: Vec<Value> = Vec::new();

    for line in input.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        if let Some(colon_pos) = line.find(':') {
            let key_raw = &line[..colon_pos];
            let val = line[colon_pos + 1..].trim().to_string();

            if line.starts_with("NEEDRESTART-VER")
                || line.starts_with("NEEDRESTART-KCUR")
                || line.starts_with("NEEDRESTART-KEXP")
                || line.starts_with("NEEDRESTART-KSTA")
                || line.starts_with("NEEDRESTART-CONT")
            {
                let k = normalize_key(key_raw);
                out.insert(k, Value::String(val));
            } else if line.starts_with("NEEDRESTART-SESS") {
                sess_list.push(Value::String(val));
            } else if line.starts_with("NEEDRESTART-SVC") {
                svc_list.push(Value::String(val));
            } else if line.starts_with("NEEDRESTART-PID") {
                pid_list.push(Value::String(val));
            }
        }
    }

    // Rename keys and convert types
    let key_map: &[(&str, &str)] = &[
        ("needrestart_ver", "version"),
        ("needrestart_kcur", "running_kernel_version"),
        ("needrestart_kexp", "expected_kernel_version"),
        ("needrestart_ksta", "kernel_status"),
        ("needrestart_cont", "container"),
    ];

    let mut final_out = Map::new();

    for (raw_key, new_key) in key_map {
        if let Some(val) = out.remove(*raw_key) {
            if *raw_key == "needrestart_ksta" {
                // Convert to integer
                if let Value::String(s) = &val
                    && let Ok(n) = s.parse::<i64>()
                {
                    final_out.insert(new_key.to_string(), Value::Number(n.into()));
                    continue;
                }
            }
            final_out.insert(new_key.to_string(), val);
        }
    }

    // Add any remaining keys not in key_map
    for (k, v) in out {
        final_out.insert(k, v);
    }

    if !sess_list.is_empty() {
        final_out.insert("session".to_string(), Value::Array(sess_list));
    }

    if !svc_list.is_empty() {
        final_out.insert("service".to_string(), Value::Array(svc_list));
    }

    if !pid_list.is_empty() {
        final_out.insert("pid".to_string(), Value::Array(pid_list));
    }

    final_out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_needrestart_basic() {
        let input = "NEEDRESTART-VER: 2.1\n\
                     NEEDRESTART-KCUR: 3.19.3-tl1+\n\
                     NEEDRESTART-KEXP: 3.19.3-tl1+\n\
                     NEEDRESTART-KSTA: 1\n\
                     NEEDRESTART-CONT: LXC web1\n\
                     NEEDRESTART-SESS: metabase @ user manager service\n\
                     NEEDRESTART-SESS: root @ session #28017\n\
                     NEEDRESTART-SVC: systemd-journald.service\n\
                     NEEDRESTART-SVC: systemd-machined.service";

        let parser = NeedrestartParser;
        let result = parser.parse(input, false).unwrap();
        if let ParseOutput::Object(obj) = result {
            assert_eq!(obj.get("version"), Some(&Value::String("2.1".to_string())));
            assert_eq!(
                obj.get("running_kernel_version"),
                Some(&Value::String("3.19.3-tl1+".to_string()))
            );
            assert_eq!(obj.get("kernel_status"), Some(&Value::Number(1.into())));
            assert_eq!(
                obj.get("container"),
                Some(&Value::String("LXC web1".to_string()))
            );
            if let Some(Value::Array(sess)) = obj.get("session") {
                assert_eq!(sess.len(), 2);
            } else {
                panic!("Expected session array");
            }
            if let Some(Value::Array(svc)) = obj.get("service") {
                assert_eq!(svc.len(), 2);
            } else {
                panic!("Expected service array");
            }
        } else {
            panic!("Expected Object");
        }
    }

    #[test]
    fn test_needrestart_empty() {
        let parser = NeedrestartParser;
        let result = parser.parse("", false).unwrap();
        if let ParseOutput::Object(obj) = result {
            assert!(obj.is_empty());
        } else {
            panic!("Expected Object");
        }
    }
}
