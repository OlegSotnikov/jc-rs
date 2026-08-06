//! Parser for `ss` command output.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use regex::Regex;
use serde_json::{Map, Value};
use std::sync::LazyLock;

// Compiled once for the process. Each of these sat inside a function the
// parser calls per line or per record, so the pattern was rebuilt from
// source for every one of them.
static RE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"\("([^"]+)",pid=(\d+),fd=(\d+)\)"#).expect("RE_RE is a valid pattern")
});

pub struct SsParser;

static INFO: ParserInfo = ParserInfo {
    name: "ss",
    argument: "--ss",
    version: "1.8.0",
    description: "Converts `ss` command output to JSON",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[Platform::Linux],
    tags: &[Tag::Command, Tag::Slurpable],
    magic_commands: &["ss"],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static SS_PARSER: SsParser = SsParser;

inventory::submit! { ParserEntry::new(&SS_PARSER) }

/// Netids that have address:port in local/peer columns
const CONTAINS_COLON: &[&str] = &["nl", "p_raw", "raw", "udp", "tcp", "v_str", "icmp6"];

/// Split a string by one or more spaces (equivalent to Python's re.split(r'[ ]{1,}', s))
fn split_one_or_more(s: &str) -> Vec<String> {
    s.split(' ')
        .filter(|p| !p.is_empty())
        .map(|p| p.to_string())
        .collect()
}

/// Split a string by two or more spaces (equivalent to Python's re.split(r'[ ]{2,}', s))
fn split_two_or_more(s: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut current = String::new();
    let mut space_count = 0;

    for ch in s.chars() {
        if ch == ' ' {
            space_count += 1;
        } else {
            if space_count >= 2 && !current.is_empty() {
                result.push(current.clone());
                current.clear();
            } else if space_count > 0 {
                // single space: keep in current token
                for _ in 0..space_count {
                    current.push(' ');
                }
            }
            space_count = 0;
            current.push(ch);
        }
    }
    if !current.trim().is_empty() {
        result.push(current);
    }
    result
}

impl Parser for SsParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Array(vec![]));
        }

        let cleandata: Vec<&str> = input.lines().filter(|l| !l.trim().is_empty()).collect();
        if cleandata.is_empty() {
            return Ok(ParseOutput::Array(vec![]));
        }

        // Parse header
        let header_raw = cleandata[0].to_lowercase();
        let recv_q_position = header_raw.find("recv-q").unwrap_or(0);

        let header_text = header_raw
            .replace("netidstate", "netid state")
            .replace("local address:port", "local_address local_port")
            .replace("peer address:port", "peer_address peer_port")
            .replace("portprocess", "port")
            .replace('-', "_");

        let mut header_list: Vec<String> = header_text
            .split_whitespace()
            .map(|s| s.to_string())
            .collect();

        let mut extra_opts = false;
        let mut result = Vec::new();

        for entry in &cleandata[1..] {
            // Skip continuation lines (start with whitespace)
            if entry.starts_with(|c: char| c.is_whitespace()) {
                continue;
            }

            let mut entry_str = entry.to_string();

            // Fix: insert "  " at recv_q_position to ensure enough space before Recv-Q
            if recv_q_position > 0 && recv_q_position < entry_str.len() {
                entry_str = format!(
                    "{}  {}",
                    &entry_str[..recv_q_position],
                    &entry_str[recv_q_position..]
                );
            }

            // Fix: insert "  " at position 5 to ensure space between first two columns
            if entry_str.len() > 5 {
                entry_str = format!("{}  {}", &entry_str[..5], &entry_str[5..]);
            }

            // Initial split by one-or-more spaces
            let trimmed = entry_str.trim();
            let mut entry_list = split_one_or_more(trimmed);

            // If too many fields or extra_opts mode: re-split by two-or-more spaces
            if entry_list.len() > header_list.len() || extra_opts {
                entry_list = split_two_or_more(trimmed);
                extra_opts = true;
            }

            // For contains_colon netids: rsplit ':' on local (idx 4) and peer (idx 6)
            if let Some(netid) = entry_list.first().map(|s| s.as_str())
                && CONTAINS_COLON.contains(&netid)
            {
                // Split local_address:port at index 4
                if let Some(local) = entry_list.get(4).cloned()
                    && local.contains(':')
                {
                    let (addr, port) = rsplit_colon(&local);
                    entry_list[4] = addr;
                    entry_list.insert(5, port);
                }
                // Split peer_address:port at index 6
                if let Some(peer) = entry_list.get(6).cloned()
                    && peer.contains(':')
                {
                    let (addr, port) = rsplit_colon(&peer);
                    entry_list[6] = addr;
                    entry_list.insert(7, port);
                }
            }

            // Check for opts at the end (matches jc's re.search for known opts patterns)
            let has_opts = entry_list
                .last()
                .map(|s| {
                    s.contains("ino:")
                        || s.contains("uid:")
                        || s.contains("sk:")
                        || s.contains("users:")
                        || s.contains("timer:")
                        || s.contains("cgroup:")
                        || s.contains("v6only:")
                })
                .unwrap_or(false);

            if has_opts && header_list.last().map(|s| s.as_str()) != Some("opts") {
                header_list.push("opts".to_string());
            }

            // Zip header with entry_list to create output_line (matches Python dict(zip(...)))
            let mut obj = Map::new();
            for (h, v) in header_list.iter().zip(entry_list.iter()) {
                let val = v.trim();
                if val.is_empty() {
                    continue;
                }
                match h.as_str() {
                    "recv_q" | "send_q" => {
                        if let Ok(n) = val.parse::<i64>() {
                            obj.insert(h.clone(), Value::Number(n.into()));
                        } else {
                            obj.insert(h.clone(), Value::String(val.to_string()));
                        }
                    }
                    "opts" => {
                        // Parse opts string into an object
                        let opts_map = parse_opts(val);
                        if !opts_map.is_empty() {
                            obj.insert(h.clone(), Value::Object(opts_map));
                        }
                    }
                    _ => {
                        obj.insert(h.clone(), Value::String(val.to_string()));
                    }
                }
            }

            // Post-process
            post_process_ss(&mut obj);

            // _process(): convert pid to int, add local_port_num, peer_port_num
            process_ss(&mut obj);

            if !obj.is_empty() {
                result.push(obj);
            }
        }

        Ok(ParseOutput::Array(result))
    }
}

/// Parse opts string into a JSON object (matches jc's _parse_opts)
fn parse_opts(opts_str: &str) -> Map<String, Value> {
    let mut opts = Map::new();

    for item in opts_str.split_whitespace() {
        // Apply key substitutions (matching jc's re.sub calls)
        // re.sub('ino', 'inode_number', re.sub('sk', 'cookie', re.sub('uid', 'uid_number', item)))
        // Applied sequentially on the whole item string
        let item = item
            .replace("ino", "inode_number")
            .replace("sk", "cookie")
            .replace("uid", "uid_number");

        if let Some(colon_pos) = item.find(':') {
            let key = &item[..colon_pos];
            let val = &item[colon_pos + 1..];

            match key {
                "users" => {
                    // Parse users:(("name",pid=N,fd=M),...) into process_id map
                    let process_map = parse_users_opts(val);
                    opts.insert("process_id".to_string(), Value::Object(process_map));
                }
                "timer" => {
                    // Parse timer:(name,expiry,retrans)
                    let timer_map = parse_timer_opts(val);
                    opts.insert("timer".to_string(), Value::Object(timer_map));
                }
                _ => {
                    if !val.is_empty() {
                        opts.insert(key.to_string(), Value::String(val.to_string()));
                    }
                }
            }
        }
    }

    opts
}

/// Parse users opts value: (("name",pid=N,fd=M),...) → {pid: {user: name, file_descriptor: fd}}
fn parse_users_opts(val: &str) -> Map<String, Value> {
    let mut result = Map::new();
    // Match each ("name",pid=N,fd=M) pattern
    let re = &*RE_RE;
    for cap in re.captures_iter(val) {
        let user = cap.get(1).map(|m| m.as_str()).unwrap_or("");
        let pid = cap.get(2).map(|m| m.as_str()).unwrap_or("");
        let fd = cap.get(3).map(|m| m.as_str()).unwrap_or("");
        let mut entry = Map::new();
        entry.insert("user".to_string(), Value::String(user.to_string()));
        entry.insert("file_descriptor".to_string(), Value::String(fd.to_string()));
        result.insert(pid.to_string(), Value::Object(entry));
    }
    result
}

/// Parse timer opts value: (name,expiry,retrans) → {timer_name, expire_time, retrans}
fn parse_timer_opts(val: &str) -> Map<String, Value> {
    let mut result = Map::new();
    // Strip outer parens and split by comma
    let inner = val.trim_start_matches('(').trim_end_matches(')');
    let parts: Vec<&str> = inner.splitn(3, ',').collect();
    if parts.len() >= 3 {
        result.insert(
            "timer_name".to_string(),
            Value::String(parts[0].trim().to_string()),
        );
        result.insert(
            "expire_time".to_string(),
            Value::String(parts[1].trim().to_string()),
        );
        result.insert(
            "retrans".to_string(),
            Value::String(parts[2].trim().to_string()),
        );
    }
    result
}

/// rsplit on last ':' (matches Python rsplit(':', maxsplit=1))
fn rsplit_colon(s: &str) -> (String, String) {
    if let Some(pos) = s.rfind(':') {
        (s[..pos].to_string(), s[pos + 1..].to_string())
    } else {
        (s.to_string(), String::new())
    }
}

/// Post-process ss entry (matches jc's inline post-processing in parse())
fn post_process_ss(obj: &mut Map<String, Value>) {
    let netid = obj
        .get("netid")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    // Handle '%' in local_address → extract interface (matches jc: if '%' in output_line['local_address'])
    if let Some(la_val) = obj.get("local_address").cloned() {
        let la = la_val.as_str().unwrap_or("");
        if la.contains('%')
            && let Some(pct) = la.rfind('%')
        {
            let addr = la[..pct].to_string();
            let iface = la[pct + 1..].to_string();
            obj.insert("local_address".to_string(), Value::String(addr));
            obj.insert("interface".to_string(), Value::String(iface));
        }
    }

    if netid == "nl" {
        // channel = local_address + ':' + local_port
        // if '/' in channel: pid = rsplit('/')[1], channel = rsplit('/')[0]
        let la = obj
            .remove("local_address")
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .unwrap_or_default();
        let lp = obj
            .remove("local_port")
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .unwrap_or_default();

        let channel_raw = format!("{}:{}", la, lp);

        let channel = if channel_raw.contains('/') {
            // rsplit('/', maxsplit=1)
            if let Some(pos) = channel_raw.rfind('/') {
                let pid_str = &channel_raw[pos + 1..];
                obj.insert("pid".to_string(), Value::String(pid_str.to_string()));
                channel_raw[..pos].to_string()
            } else {
                channel_raw
            }
        } else {
            channel_raw
        };

        obj.insert("channel".to_string(), Value::String(channel));
        return;
    }

    if netid == "p_raw" {
        // link_layer = local_address, interface = local_port
        let la = obj
            .remove("local_address")
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .unwrap_or_default();
        let lp = obj
            .remove("local_port")
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .unwrap_or_default();
        if !la.is_empty() {
            obj.insert("link_layer".to_string(), Value::String(la));
        }
        if !lp.is_empty() {
            obj.insert("interface".to_string(), Value::String(lp));
        }
        return;
    }

    // Not in contains_colon: local_address → path
    if !CONTAINS_COLON.contains(&netid.as_str()) {
        let la = obj
            .remove("local_address")
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .unwrap_or_default();
        if !la.is_empty() {
            obj.insert("path".to_string(), Value::String(la));
        }
    }
}

/// _process(): convert pid to int; add local_port_num and peer_port_num
fn process_ss(obj: &mut Map<String, Value>) {
    // Convert pid to int
    if let Some(pid_val) = obj.get("pid").cloned() {
        let s = pid_val.as_str().unwrap_or("");
        if let Ok(n) = s.parse::<i64>() {
            obj.insert("pid".to_string(), Value::Number(n.into()));
        }
    }

    // local_port_num
    if let Some(lp_val) = obj.get("local_port").cloned() {
        let s = lp_val.as_str().unwrap_or("");
        if let Ok(n) = s.parse::<i64>()
            && n >= 0
        {
            obj.insert("local_port_num".to_string(), Value::Number(n.into()));
        }
    }

    // peer_port_num
    if let Some(pp_val) = obj.get("peer_port").cloned() {
        let s = pp_val.as_str().unwrap_or("");
        if let Ok(n) = s.parse::<i64>()
            && n >= 0
        {
            obj.insert("peer_port_num".to_string(), Value::Number(n.into()));
        }
    }
}
