//! Windows `net localgroup` command parser.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};

struct NetLocalgroupParser;

static INFO: ParserInfo = ParserInfo {
    name: "net_localgroup",
    argument: "--net-localgroup",
    version: "1.0.0",
    description: "Windows `net localgroup` command parser",
    author: "jc-rs contributors",
    author_email: "os@g1sw.com",
    compatible: &[Platform::Windows],
    tags: &[Tag::Command],
    magic_commands: &["net localgroup"],
    streaming: false,
    hidden: false,
    deprecated: false,
};

pub fn parse_net_localgroup(input: &str) -> Map<String, Value> {
    let mut obj = Map::new();
    let mut account_origin: Option<String> = None;
    let domain: Option<String> = None;
    let mut comment: Option<String> = None;
    let mut groups: Vec<Value> = Vec::new();
    let mut members: Vec<Value> = Vec::new();
    let mut current_group: Option<String> = None;
    let mut in_members = false;
    let mut in_group_detail = false;

    for line in input.lines() {
        let trimmed = line.trim();

        // Skip separator lines
        if trimmed.starts_with("---") {
            continue;
        }

        // Skip "The command completed successfully." and empty lines
        if trimmed.is_empty() || trimmed.starts_with("The command completed") {
            continue;
        }

        // "Aliases for \\MACHINE": list mode
        if trimmed.starts_with("Aliases for ") {
            let origin = trimmed["Aliases for ".len()..].trim();
            account_origin = Some(origin.to_string());
            in_group_detail = false;
            continue;
        }

        // "Alias name     GroupName": detail mode header
        if trimmed.starts_with("Alias name") {
            let group_name = trimmed["Alias name".len()..].trim().to_string();
            current_group = Some(group_name);
            in_group_detail = true;
            in_members = false;
            continue;
        }

        // "Comment        ...": detail mode
        if in_group_detail && trimmed.starts_with("Comment") {
            let val = trimmed["Comment".len()..].trim();
            comment = if val.is_empty() {
                None
            } else {
                Some(val.to_string())
            };
            continue;
        }

        // "Members" header in detail mode
        if in_group_detail && trimmed == "Members" {
            in_members = true;
            continue;
        }

        // In detail mode, collect members
        if in_group_detail && in_members && !trimmed.is_empty() {
            members.push(Value::String(trimmed.to_string()));
            continue;
        }

        // In list mode: "*GroupName" lines
        if !in_group_detail && trimmed.starts_with('*') {
            let group_name = trimmed[1..].trim().to_string();
            let group_obj = serde_json::json!({
                "name": group_name,
                "members": []
            });
            groups.push(group_obj);
            continue;
        }

        // "User accounts for \\MACHINE" or "Group name ...": alternate formats
        if trimmed.starts_with("Group name") {
            let group_name = trimmed["Group name".len()..].trim().to_string();
            current_group = Some(group_name);
            in_group_detail = true;
            in_members = false;
            continue;
        }
    }

    // Finalize detail mode result
    if in_group_detail && let Some(name) = current_group {
        let members_arr = Value::Array(members);
        let group_obj = serde_json::json!({
            "name": name,
            "members": members_arr
        });
        groups.push(group_obj);
    }

    obj.insert(
        "account_origin".to_string(),
        match account_origin {
            Some(s) => Value::String(s),
            None => Value::Null,
        },
    );
    obj.insert(
        "domain".to_string(),
        match domain {
            Some(s) => Value::String(s),
            None => Value::Null,
        },
    );
    obj.insert(
        "comment".to_string(),
        match comment {
            Some(s) => Value::String(s),
            None => Value::Null,
        },
    );
    obj.insert("groups".to_string(), Value::Array(groups));

    obj
}

impl Parser for NetLocalgroupParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        Ok(ParseOutput::Object(parse_net_localgroup(input)))
    }
}

static INSTANCE: NetLocalgroupParser = NetLocalgroupParser;

inventory::submit! {
    ParserEntry::new(&INSTANCE)
}

#[cfg(test)]
mod tests {
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
    fn test_net_localgroup_registered() {
        assert!(find_parser("net_localgroup").is_some());
    }

    #[test]
    fn test_net_localgroup_list_mode() {
        let input = get_fixture("windows/windows-10/net_localgroup.out");
        let parser = find_parser("net_localgroup").unwrap();
        let result = parser.parse(&input, true).unwrap();
        let obj = match result {
            ParseOutput::Object(o) => o,
            _ => panic!("expected object"),
        };
        assert_eq!(
            obj["account_origin"],
            serde_json::json!("\\\\DESKTOP-WIN10-PRO")
        );
        assert!(obj["domain"].is_null());
        let groups = obj["groups"].as_array().unwrap();
        assert!(!groups.is_empty());
        // Check first group
        assert_eq!(
            groups[0]["name"],
            serde_json::json!("Access Control Assistance Operators")
        );
        assert_eq!(groups[0]["members"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn test_net_localgroup_detail_mode() {
        let input = get_fixture("windows/windows-10/net_localgroup.administrators.out");
        let parser = find_parser("net_localgroup").unwrap();
        let result = parser.parse(&input, true).unwrap();
        let obj = match result {
            ParseOutput::Object(o) => o,
            _ => panic!("expected object"),
        };
        assert!(obj["account_origin"].is_null());
        let comment = obj["comment"].as_str().unwrap();
        assert!(comment.contains("Administrators"));
        let groups = obj["groups"].as_array().unwrap();
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0]["name"], serde_json::json!("Administrators"));
        let members = groups[0]["members"].as_array().unwrap();
        assert_eq!(members.len(), 2);
        assert!(members.contains(&serde_json::json!("Administrator")));
        assert!(members.contains(&serde_json::json!("user1")));
    }
}
