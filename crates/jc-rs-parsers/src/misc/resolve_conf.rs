use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};

pub struct ResolveConfParser;

static INFO: ParserInfo = ParserInfo {
    name: "resolve_conf",
    argument: "--resolve-conf",
    version: "1.0.0",
    description: "Converts `/etc/resolv.conf` file content to JSON",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[
        Platform::Linux,
        Platform::Darwin,
        Platform::FreeBSD,
        Platform::Windows,
    ],
    tags: &[Tag::File],
    magic_commands: &[],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static RESOLVE_CONF_PARSER: ResolveConfParser = ResolveConfParser;

inventory::submit! {
    ParserEntry::new(&RESOLVE_CONF_PARSER)
}

impl Parser for ResolveConfParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Object(Map::new()));
        }

        let mut obj = Map::new();
        let mut search: Vec<Value> = Vec::new();
        let mut nameservers: Vec<Value> = Vec::new();
        let mut options: Vec<Value> = Vec::new();
        let mut sortlist: Vec<Value> = Vec::new();

        for line in input.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            // Strip comments: split on # or ;
            let userdata_str = if let Some(pos) = line.find(|c| c == '#' || c == ';') {
                let before_comment = line[..pos].trim();
                if before_comment.is_empty() {
                    continue;
                }
                before_comment
            } else {
                line
            };

            let parts: Vec<&str> = userdata_str.splitn(2, char::is_whitespace).collect();
            if parts.is_empty() {
                continue;
            }

            match parts[0] {
                "domain" => {
                    if let Some(val) = parts.get(1) {
                        obj.insert(
                            "domain".to_string(),
                            Value::String(val.split_whitespace().next().unwrap_or("").to_string()),
                        );
                    }
                }
                "search" => {
                    if let Some(val) = parts.get(1) {
                        for item in val.split_whitespace() {
                            search.push(Value::String(item.to_string()));
                        }
                    }
                }
                "nameserver" => {
                    if let Some(val) = parts.get(1) {
                        if let Some(ns) = val.split_whitespace().next() {
                            nameservers.push(Value::String(ns.to_string()));
                        }
                    }
                }
                "options" => {
                    if let Some(val) = parts.get(1) {
                        for item in val.split_whitespace() {
                            options.push(Value::String(item.to_string()));
                        }
                    }
                }
                "sortlist" => {
                    if let Some(val) = parts.get(1) {
                        for item in val.split_whitespace() {
                            sortlist.push(Value::String(item.to_string()));
                        }
                    }
                }
                _ => {}
            }
        }

        if !search.is_empty() {
            obj.insert("search".to_string(), Value::Array(search));
        }
        if !nameservers.is_empty() {
            obj.insert("nameservers".to_string(), Value::Array(nameservers));
        }
        if !options.is_empty() {
            obj.insert("options".to_string(), Value::Array(options));
        }
        if !sortlist.is_empty() {
            obj.insert("sortlist".to_string(), Value::Array(sortlist));
        }

        Ok(ParseOutput::Object(obj))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! resolve_conf_test {
        ($name:ident, $input:expr, $expected:expr) => {
            #[test]
            fn $name() {
                let input = include_str!($input);
                let expected: serde_json::Value =
                    serde_json::from_str(include_str!($expected)).unwrap();
                let parser = ResolveConfParser;
                let result = parser.parse(input, false).unwrap();
                let result_val = serde_json::to_value(result).unwrap();
                assert_eq!(result_val, expected);
            }
        };
    }

    resolve_conf_test!(
        test_resolve_conf_1,
        "../../../../tests/fixtures/generic/resolve.conf-1",
        "../../../../tests/fixtures/generic/resolve.conf-1.json"
    );
    resolve_conf_test!(
        test_resolve_conf_2,
        "../../../../tests/fixtures/generic/resolve.conf-2",
        "../../../../tests/fixtures/generic/resolve.conf-2.json"
    );
    resolve_conf_test!(
        test_resolve_conf_3,
        "../../../../tests/fixtures/generic/resolve.conf-3",
        "../../../../tests/fixtures/generic/resolve.conf-3.json"
    );
    resolve_conf_test!(
        test_resolve_conf_4,
        "../../../../tests/fixtures/generic/resolve.conf-4",
        "../../../../tests/fixtures/generic/resolve.conf-4.json"
    );
}
