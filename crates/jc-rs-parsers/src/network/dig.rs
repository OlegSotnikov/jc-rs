//! Parser for `dig` command output.
//!
//! Implements a state-machine parser matching jc's dig.py logic.
//! Supports standard output, +noall +answer, and +axfr.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use jc_rs_utils::parse_timestamp;
use serde_json::{Map, Value};

pub struct DigParser;

static INFO: ParserInfo = ParserInfo {
    name: "dig",
    argument: "--dig",
    version: "2.5.0",
    description: "Converts `dig` command output to JSON",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[
        Platform::Linux,
        Platform::Darwin,
        Platform::FreeBSD,
        Platform::Windows,
    ],
    tags: &[Tag::Command, Tag::Slurpable],
    magic_commands: &["dig"],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static DIG_PARSER: DigParser = DigParser;

inventory::submit! { ParserEntry::new(&DIG_PARSER) }

#[derive(Debug, PartialEq, Clone)]
enum Section {
    None,
    Header,
    Flags,
    OptPseudosection,
    Question,
    Answer,
    Authority,
    Additional,
    Footer,
    Axfr,
}

fn parse_header(line: &str) -> Map<String, Value> {
    // ;; ->>HEADER<<- opcode: QUERY, status: NXDOMAIN, id: 6140
    // parts: ";;" "->>HEADER<<-" "opcode:" "QUERY," "status:" "NOERROR," "id:" "6140"
    //          0         1           2         3         4         5        6     7
    let mut obj = Map::new();
    let parts: Vec<&str> = line.split_whitespace().collect();

    // Find "opcode:" keyword and take the next token
    if let Some(idx) = parts.iter().position(|&p| p.starts_with("opcode:")) {
        // opcode may be "opcode:" separate or "opcode:QUERY,"
        let opcode_val = if p_after(&parts, idx).is_empty() {
            parts[idx]["opcode:".len()..]
                .trim_end_matches(',')
                .to_string()
        } else {
            p_after(&parts, idx).trim_end_matches(',').to_string()
        };
        obj.insert("opcode".to_string(), Value::String(opcode_val));
    }

    if let Some(idx) = parts.iter().position(|&p| p.starts_with("status:")) {
        let status_val = if p_after(&parts, idx).is_empty() {
            parts[idx]["status:".len()..]
                .trim_end_matches(',')
                .to_string()
        } else {
            p_after(&parts, idx).trim_end_matches(',').to_string()
        };
        obj.insert("status".to_string(), Value::String(status_val));
    }

    if let Some(idx) = parts.iter().position(|&p| p.starts_with("id:")) {
        let id_val = if parts[idx].len() > 3 {
            &parts[idx][3..]
        } else {
            p_after_ref(&parts, idx)
        };
        if let Ok(n) = id_val.parse::<i64>() {
            obj.insert("id".to_string(), Value::Number(n.into()));
        }
    }

    obj
}

fn p_after<'a>(parts: &[&'a str], idx: usize) -> String {
    parts.get(idx + 1).copied().unwrap_or("").to_string()
}

fn p_after_ref<'a>(parts: &[&'a str], idx: usize) -> &'a str {
    parts.get(idx + 1).copied().unwrap_or("")
}

fn parse_flags_line(line: &str) -> Map<String, Value> {
    // ;; flags: qr rd ra; QUERY: 1, ANSWER: 0, AUTHORITY: 0, ADDITIONAL: 1
    let mut obj = Map::new();

    // Extract flags between "flags:" and ";"
    let flags_part = if let Some(after_flags) = line.find("flags:") {
        let after = &line[after_flags + 6..];
        if let Some(semi) = after.find(';') {
            after[..semi].trim().to_string()
        } else {
            String::new()
        }
    } else {
        String::new()
    };

    let flags: Vec<Value> = flags_part
        .split_whitespace()
        .map(|f| Value::String(f.to_string()))
        .collect();
    obj.insert("flags".to_string(), Value::Array(flags));

    // Extract counts
    let rest = line.replace(',', " ").replace(':', " ");
    let parts: Vec<&str> = rest.split_whitespace().collect();
    let find_after = |needle: &str| -> Option<i64> {
        parts
            .iter()
            .position(|&p| p.to_uppercase() == needle.to_uppercase())
            .and_then(|i| parts.get(i + 1))
            .and_then(|v| v.parse::<i64>().ok())
    };

    if let Some(n) = find_after("QUERY") {
        obj.insert("query_num".to_string(), Value::Number(n.into()));
    }
    if let Some(n) = find_after("ANSWER") {
        obj.insert("answer_num".to_string(), Value::Number(n.into()));
    }
    if let Some(n) = find_after("AUTHORITY") {
        obj.insert("authority_num".to_string(), Value::Number(n.into()));
    }
    if let Some(n) = find_after("ADDITIONAL") {
        obj.insert("additional_num".to_string(), Value::Number(n.into()));
    }

    obj
}

fn parse_opt_pseudosection(line: &str) -> Map<String, Value> {
    let mut obj = Map::new();

    if line.starts_with("; EDNS:") {
        // ; EDNS: version: 0, flags:; udp: 4096
        let cleaned = line.replace(',', " ");
        let parts: Vec<&str> = cleaned.split(';').collect();
        let first_part = parts.get(1).unwrap_or(&"");
        let rest_part = parts.get(2).unwrap_or(&"");

        let fp_tokens: Vec<&str> = first_part.split_whitespace().collect();
        // tokens: ["EDNS:", "version:", "0", "flags:"]
        let version = fp_tokens.get(2).and_then(|v| v.parse::<i64>().ok());
        // flags come after "flags:" token
        let flags_idx = fp_tokens.iter().position(|&t| t.starts_with("flags:"));
        let flags: Vec<Value> = if let Some(idx) = flags_idx {
            fp_tokens[idx + 1..]
                .iter()
                .map(|&f| Value::String(f.to_string()))
                .collect()
        } else {
            vec![]
        };

        let udp_tokens: Vec<&str> = rest_part.split_whitespace().collect();
        let udp = udp_tokens.last().and_then(|v| v.parse::<i64>().ok());

        let mut edns = Map::new();
        if let Some(v) = version {
            edns.insert("version".to_string(), Value::Number(v.into()));
        }
        edns.insert("flags".to_string(), Value::Array(flags));
        if let Some(u) = udp {
            edns.insert("udp".to_string(), Value::Number(u.into()));
        }

        obj.insert("edns".to_string(), Value::Object(edns));
    } else if line.starts_with("; COOKIE:") {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if let Some(cookie) = parts.get(2) {
            obj.insert("cookie".to_string(), Value::String(cookie.to_string()));
        }
    } else if line.starts_with("; NSID:") {
        // ; NSID: 67 70 64 6e 73 2d 73 66 6f ("gpdns-sfo")
        let nsid = if let Some(start) = line.find("(\"") {
            line[start + 2..].trim_end_matches("\")").to_string()
        } else {
            String::new()
        };
        obj.insert("nsid".to_string(), Value::String(nsid));
    }

    obj
}

fn parse_question(line: &str) -> Map<String, Value> {
    // ;www.cnn.com.           IN  A
    let parts: Vec<&str> = line.split_whitespace().collect();
    let mut obj = Map::new();
    if parts.len() >= 3 {
        obj.insert(
            "name".to_string(),
            Value::String(parts[0].trim_start_matches(';').to_string()),
        );
        obj.insert("class".to_string(), Value::String(parts[1].to_string()));
        obj.insert("type".to_string(), Value::String(parts[2].to_string()));
    }
    obj
}

fn parse_answer_record(line: &str) -> Map<String, Value> {
    // www.cnn.com.   5   IN  CNAME   turner-tls.map.fastly.net.
    let parts: Vec<&str> = line.split_whitespace().collect();
    let mut obj = Map::new();
    if parts.len() >= 5 {
        let name = parts[0].to_string();
        let ttl_str = parts[1];
        let class = parts[2].to_string();
        let rtype = parts[3].to_string();
        // data is everything from position 4 onward (rejoin with spaces)
        let data = parts[4..].join(" ");

        obj.insert("name".to_string(), Value::String(name));
        if let Ok(ttl) = ttl_str.parse::<i64>() {
            obj.insert("ttl".to_string(), Value::Number(ttl.into()));
        } else {
            obj.insert("ttl".to_string(), Value::String(ttl_str.to_string()));
        }
        obj.insert("class".to_string(), Value::String(class));
        obj.insert("type".to_string(), Value::String(rtype));
        obj.insert("data".to_string(), Value::String(data));
    }
    obj
}

fn parse_axfr_record(line: &str) -> Map<String, Value> {
    // zonetransfer.me. 7200 IN A 5.196.105.14
    parse_answer_record(line)
}

fn parse_footer(line: &str) -> Option<(String, Value)> {
    if line.starts_with(";; Query time:") {
        // ";; Query time: 49 msec" -> extract integer
        let after = line.splitn(2, ':').nth(1).unwrap_or("").trim();
        // after = "49 msec"
        let num_str = after.split_whitespace().next().unwrap_or("");
        if let Ok(n) = num_str.parse::<i64>() {
            return Some(("query_time".to_string(), Value::Number(n.into())));
        } else {
            return Some(("query_time".to_string(), Value::String(after.to_string())));
        }
    }
    if line.starts_with(";; SERVER:") {
        // reassemble: SERVER: host:port
        let full = line[";;".len()..]
            .trim()
            .trim_start_matches("SERVER:")
            .trim()
            .to_string();
        return Some(("server".to_string(), Value::String(full)));
    }
    if line.starts_with(";; WHEN:") {
        let val = line[";;".len()..]
            .trim()
            .trim_start_matches("WHEN:")
            .trim()
            .to_string();
        return Some(("when".to_string(), Value::String(val)));
    }
    if line.starts_with(";; MSG SIZE  rcvd:") {
        let val = line.splitn(2, ':').nth(1).unwrap_or("").trim().to_string();
        if let Ok(n) = val.parse::<i64>() {
            return Some(("rcvd".to_string(), Value::Number(n.into())));
        }
    }
    if line.starts_with(";; XFR size:") {
        let val = line.splitn(2, ':').nth(1).unwrap_or("").trim().to_string();
        return Some(("size".to_string(), Value::String(val)));
    }
    if line.starts_with(";; QUERY SIZE:") {
        let val = line.splitn(2, ':').nth(1).unwrap_or("").trim().to_string();
        if let Ok(n) = val.parse::<i64>() {
            return Some(("query_size".to_string(), Value::Number(n.into())));
        }
    }
    None
}

impl Parser for DigParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Array(vec![]));
        }

        let cleandata: Vec<&str> = input.lines().filter(|l| !l.trim().is_empty()).collect();

        let mut raw_output: Vec<Map<String, Value>> = Vec::new();
        let mut section = Section::None;
        let mut output_entry: Map<String, Value> = Map::new();
        let mut answer_list: Vec<Value> = Vec::new();
        let mut authority_list: Vec<Value> = Vec::new();
        let mut additional_list: Vec<Value> = Vec::new();
        let mut axfr_list: Vec<Value> = Vec::new();
        let mut opt_section: Map<String, Value> = Map::new();

        for line in &cleandata {
            // Identify sections
            if line.starts_with(";; Got answer:") {
                section = Section::None;
                continue;
            }

            if line.starts_with("; <<>> ") && line.to_lowercase().contains(" axfr ") {
                section = Section::Axfr;
                axfr_list = Vec::new();
                continue;
            }

            if line.starts_with(";; ->>HEADER<<-") {
                // Save previous entry if any
                if !output_entry.is_empty() {
                    if !answer_list.is_empty() {
                        output_entry
                            .insert("answer".to_string(), Value::Array(answer_list.clone()));
                    }
                    if !authority_list.is_empty() {
                        output_entry.insert(
                            "authority".to_string(),
                            Value::Array(authority_list.clone()),
                        );
                    }
                    if !additional_list.is_empty() {
                        output_entry.insert(
                            "additional".to_string(),
                            Value::Array(additional_list.clone()),
                        );
                    }
                    if !opt_section.is_empty() {
                        output_entry.insert(
                            "opt_pseudosection".to_string(),
                            Value::Object(opt_section.clone()),
                        );
                    }
                    raw_output.push(output_entry.clone());
                }
                output_entry = Map::new();
                answer_list = Vec::new();
                authority_list = Vec::new();
                additional_list = Vec::new();
                axfr_list = Vec::new();
                opt_section = Map::new();

                section = Section::Header;
                let header_data = parse_header(line);
                output_entry.extend(header_data);
                continue;
            }

            if line.starts_with(";; flags:") {
                section = Section::Flags;
                let flags_data = parse_flags_line(line);
                output_entry.extend(flags_data);
                continue;
            }

            if line.starts_with(";; OPT PSEUDOSECTION:") {
                section = Section::OptPseudosection;
                continue;
            }

            if line.starts_with(";; QUESTION SECTION:") {
                section = Section::Question;
                continue;
            }

            if line.starts_with(";; AUTHORITY SECTION:") {
                section = Section::Authority;
                continue;
            }

            if line.starts_with(";; ANSWER SECTION:") {
                section = Section::Answer;
                continue;
            }

            if line.starts_with(";; ADDITIONAL SECTION:") {
                section = Section::Additional;
                continue;
            }

            if line.starts_with(";; Query time:") {
                section = Section::Footer;
                if let Some((k, v)) = parse_footer(line) {
                    output_entry.insert(k, v);
                }
                continue;
            }

            // Parse QUERY SIZE inline
            if line.starts_with(";; QUERY SIZE:") {
                if let Some((k, v)) = parse_footer(line) {
                    output_entry.insert(k, v);
                }
                continue;
            }

            // Parse content by section
            if !line.starts_with(';') && section == Section::Axfr {
                axfr_list.push(Value::Object(parse_axfr_record(line)));
                output_entry.insert("axfr".to_string(), Value::Array(axfr_list.clone()));
                continue;
            }

            if section == Section::OptPseudosection && line.starts_with("; ") {
                let opt_data = parse_opt_pseudosection(line);
                opt_section.extend(opt_data);
                continue;
            }

            if section == Section::Question {
                if line.starts_with(';') && !line.starts_with(";;") {
                    output_entry
                        .insert("question".to_string(), Value::Object(parse_question(line)));
                }
                continue;
            }

            if !line.starts_with(';') && section == Section::Authority {
                authority_list.push(Value::Object(parse_answer_record(line)));
                output_entry.insert(
                    "authority".to_string(),
                    Value::Array(authority_list.clone()),
                );
                continue;
            }

            if !line.starts_with(';') && (section == Section::Answer || section == Section::None) {
                answer_list.push(Value::Object(parse_answer_record(line)));
                output_entry.insert("answer".to_string(), Value::Array(answer_list.clone()));
                continue;
            }

            if !line.starts_with(';') && section == Section::Additional {
                additional_list.push(Value::Object(parse_answer_record(line)));
                output_entry.insert(
                    "additional".to_string(),
                    Value::Array(additional_list.clone()),
                );
                continue;
            }

            if section == Section::Footer {
                if let Some((k, v)) = parse_footer(line) {
                    output_entry.insert(k, v);
                }
                continue;
            }
        }

        // Flush last entry
        if !output_entry.is_empty() {
            if !opt_section.is_empty() {
                output_entry.insert("opt_pseudosection".to_string(), Value::Object(opt_section));
            }
            raw_output.push(output_entry);
        }

        // Filter empty entries
        let mut raw_output: Vec<Map<String, Value>> =
            raw_output.into_iter().filter(|e| !e.is_empty()).collect();

        // Post-process: compute when_epoch and when_epoch_utc from "when" field
        for entry in &mut raw_output {
            if let Some(Value::String(when_str)) = entry.get("when") {
                let parsed = parse_timestamp(when_str, None);
                entry.insert(
                    "when_epoch".to_string(),
                    parsed
                        .naive_epoch
                        .map(|e| Value::Number(e.into()))
                        .unwrap_or(Value::Null),
                );
                entry.insert(
                    "when_epoch_utc".to_string(),
                    parsed
                        .utc_epoch
                        .map(|e| Value::Number(e.into()))
                        .unwrap_or(Value::Null),
                );
            }
        }

        Ok(ParseOutput::Array(raw_output))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jc_rs_core::traits::Parser;

    #[test]
    fn test_dig_centos_golden() {
        let input = include_str!("../../../../tests/fixtures/centos-7.7/dig.out");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/centos-7.7/dig.json"
        ))
        .unwrap();
        let result = DigParser.parse(input, false).unwrap();
        let actual = serde_json::to_value(result).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_dig_centos_aaaa_golden() {
        let input = include_str!("../../../../tests/fixtures/centos-7.7/dig-aaaa.out");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/centos-7.7/dig-aaaa.json"
        ))
        .unwrap();
        let result = DigParser.parse(input, false).unwrap();
        let actual = serde_json::to_value(result).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_dig_empty() {
        let result = DigParser.parse("", false).unwrap();
        assert!(matches!(result, ParseOutput::Array(v) if v.is_empty()));
    }

    #[test]
    fn test_dig_registered() {
        assert!(jc_rs_core::registry::find_parser("dig").is_some());
    }
}
