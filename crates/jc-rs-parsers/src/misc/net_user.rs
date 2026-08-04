//! Windows `net user` command parser.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use jc_rs_utils::{parse_timestamp, timestamp::formats};
use serde_json::{Map, Value};

struct NetUserParser;

static INFO: ParserInfo = ParserInfo {
    name: "net_user",
    argument: "--net-user",
    version: "1.0.0",
    description: "Windows `net user` command parser",
    author: "jc-rs contributors",
    author_email: "jc-rs@example.com",
    compatible: &[Platform::Windows],
    tags: &[Tag::Command],
    magic_commands: &["net user"],
    streaming: false,
    hidden: false,
    deprecated: false,
};

/// Try to parse a date string used in net user output to ISO 8601.
fn parse_net_date(s: &str) -> String {
    let s = s.trim();
    // Try common Windows date formats
    for hint in [&[formats::F1700][..], &[formats::F1600][..]] {
        let parsed = parse_timestamp(s, hint);
        if let Some(iso) = parsed.iso {
            return iso;
        }
    }
    s.to_string()
}

fn parse_yes_no(s: &str) -> bool {
    let lower = s.trim().to_lowercase();
    lower == "yes"
}

fn parse_active(s: &str) -> bool {
    let lower = s.trim().to_lowercase();
    lower == "yes"
}

/// Parse group memberships from "  *Group1  *Group2  *Group3" format.
fn parse_group_memberships(s: &str) -> Vec<Value> {
    s.split('*')
        .map(|g| g.trim())
        .filter(|g| !g.is_empty() && *g != "None")
        .map(|g| Value::String(g.to_string()))
        .collect()
}

/// Parse the "User accounts for \\MACHINE" list mode.
fn parse_list_mode(input: &str) -> Map<String, Value> {
    let mut obj = Map::new();
    let mut account_origin: Option<String> = None;
    let mut user_accounts: Vec<Value> = Vec::new();

    for line in input.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty()
            || trimmed.starts_with("---")
            || trimmed.starts_with("The command completed")
        {
            continue;
        }

        if trimmed.starts_with("User accounts for ") {
            let origin = trimmed["User accounts for ".len()..].trim();
            account_origin = Some(origin.to_string());
            continue;
        }

        // User account names are space-separated on each line (up to 3 columns)
        // Each name is separated by multiple spaces
        for name in trimmed.split_whitespace() {
            let user = serde_json::json!({"user_name": name});
            user_accounts.push(user);
        }
    }

    obj.insert("domain".to_string(), Value::String(String::new()));
    obj.insert("user_accounts".to_string(), Value::Array(user_accounts));
    obj.insert(
        "account_origin".to_string(),
        match account_origin {
            Some(s) => Value::String(s),
            None => Value::Null,
        },
    );
    obj
}

/// Parse a single user detail section.
fn parse_user_detail(lines: &[&str]) -> Map<String, Value> {
    let mut user = Map::new();
    let mut local_memberships = String::new();
    let mut global_memberships = String::new();

    for line in lines {
        // Split on column alignment: label is left-padded, value is right part
        // Lines are formatted as "Label                    Value"
        // where label ends around column 29
        if line.len() < 2 {
            continue;
        }

        // Try to split: find first run of 2+ spaces after the label
        let parts: Vec<&str> = line.splitn(2, "  ").collect();
        if parts.len() < 2 {
            continue;
        }
        let label = parts[0].trim();
        let value = parts[1].trim();

        match label {
            "User name" => {
                user.insert("user_name".to_string(), Value::String(value.to_string()));
            }
            "Full Name" => {
                if !value.is_empty() {
                    user.insert("full_name".to_string(), Value::String(value.to_string()));
                }
            }
            "Comment" => {
                if !value.is_empty() {
                    user.insert("comment".to_string(), Value::String(value.to_string()));
                }
            }
            "User's comment" => {
                if !value.is_empty() {
                    user.insert(
                        "users_comment".to_string(),
                        Value::String(value.to_string()),
                    );
                }
            }
            "Country/region code" | "Country code" | "Country/Region Code" => {
                user.insert(
                    "country_region_code".to_string(),
                    Value::String(value.to_string()),
                );
            }
            "Account active" => {
                user.insert(
                    "account_active".to_string(),
                    Value::Bool(parse_active(value)),
                );
            }
            "Account expires" => {
                user.insert(
                    "account_expires".to_string(),
                    Value::String(value.to_string()),
                );
            }
            "Password last set" => {
                if !value.is_empty() {
                    user.insert(
                        "password_last_set".to_string(),
                        Value::String(parse_net_date(value)),
                    );
                }
            }
            "Password expires" => {
                user.insert(
                    "password_expires".to_string(),
                    Value::String(value.to_string()),
                );
            }
            "Password changeable" => {
                if !value.is_empty() {
                    user.insert(
                        "password_changeable".to_string(),
                        Value::String(parse_net_date(value)),
                    );
                }
            }
            "Password required" => {
                user.insert(
                    "password_required".to_string(),
                    Value::Bool(parse_yes_no(value)),
                );
            }
            "User may change password" => {
                user.insert(
                    "user_may_change_password".to_string(),
                    Value::Bool(parse_yes_no(value)),
                );
            }
            "Workstations allowed" => {
                user.insert(
                    "workstations_allowed".to_string(),
                    Value::String(value.to_string()),
                );
            }
            "Last logon" => {
                user.insert("last_logon".to_string(), Value::String(value.to_string()));
            }
            "Logon hours allowed" => {
                user.insert(
                    "logon_hours_allowed".to_string(),
                    Value::String(value.to_string()),
                );
            }
            "Local Group Memberships" => {
                local_memberships = value.to_string();
            }
            "Global Group memberships" | "Global Group Memberships" => {
                global_memberships = value.to_string();
            }
            _ => {}
        }
    }

    // Also collect continuation lines for group memberships (multi-line)
    user.insert(
        "local_group_memberships".to_string(),
        Value::Array(parse_group_memberships(&local_memberships)),
    );
    user.insert(
        "global_group_memberships".to_string(),
        Value::Array(parse_group_memberships(&global_memberships)),
    );

    user
}

pub fn parse_net_user(input: &str) -> Map<String, Value> {
    let lines: Vec<&str> = input.lines().collect();

    // Detect mode: "User accounts for" = list mode
    let is_list_mode = lines
        .iter()
        .any(|l| l.trim().starts_with("User accounts for "));

    if is_list_mode {
        return parse_list_mode(input);
    }

    // Detail mode: parse single user
    let mut obj = Map::new();
    let mut user_lines: Vec<&str> = Vec::new();

    for line in &lines {
        let trimmed = line.trim();
        if trimmed.starts_with("---")
            || trimmed.is_empty()
            || trimmed.starts_with("The command completed")
        {
            continue;
        }
        user_lines.push(line);
    }

    let user = parse_user_detail(&user_lines);
    obj.insert("domain".to_string(), Value::String(String::new()));
    obj.insert(
        "user_accounts".to_string(),
        Value::Array(vec![Value::Object(user)]),
    );
    obj
}

impl Parser for NetUserParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        Ok(ParseOutput::Object(parse_net_user(input)))
    }
}

static INSTANCE: NetUserParser = NetUserParser;

inventory::submit! {
    ParserEntry::new(&INSTANCE)
}

#[cfg(test)]
mod tests {
    use super::*;
    use jc_rs_core::registry::find_parser;
    use jc_rs_core::types::ParseOutput;

    fn get_fixture(rel_path: &str) -> String {
        let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default();
        let paths = [
            format!("{manifest}/../../tests/fixtures/{rel_path}"),
            format!("{manifest}/../../../tests/fixtures/{rel_path}"),
        ];
        for p in &paths {
            if let Ok(c) = std::fs::read_to_string(p) {
                return c;
            }
        }
        panic!("fixture not found: {rel_path}");
    }

    #[test]
    fn test_net_user_registered() {
        assert!(find_parser("net_user").is_some());
    }

    #[test]
    fn test_net_user_list_mode() {
        let input = get_fixture("windows/windows-10/net_user.out");
        let parser = find_parser("net_user").unwrap();
        let result = parser.parse(&input, true).unwrap();
        let obj = match result {
            ParseOutput::Object(o) => o,
            _ => panic!("expected object"),
        };
        assert_eq!(
            obj["account_origin"],
            serde_json::json!("\\\\DESKTOP-WIN10-PRO")
        );
        let users = obj["user_accounts"].as_array().unwrap();
        assert_eq!(users.len(), 6);
        assert!(users.iter().any(|u| u["user_name"] == "Administrator"));
    }

    #[test]
    fn test_net_user_detail_mode() {
        let input = get_fixture("windows/windows-10/net_user.administrator.out");
        let parser = find_parser("net_user").unwrap();
        let result = parser.parse(&input, true).unwrap();
        let obj = match result {
            ParseOutput::Object(o) => o,
            _ => panic!("expected object"),
        };
        let users = obj["user_accounts"].as_array().unwrap();
        assert_eq!(users.len(), 1);
        let user = &users[0];
        assert_eq!(user["user_name"], serde_json::json!("Administrator"));
        assert_eq!(user["account_active"], serde_json::json!(false));
        let local_groups = user["local_group_memberships"].as_array().unwrap();
        assert!(local_groups.contains(&serde_json::json!("Administrators")));
        let global_groups = user["global_group_memberships"].as_array().unwrap();
        assert!(global_groups.is_empty());
    }
}
