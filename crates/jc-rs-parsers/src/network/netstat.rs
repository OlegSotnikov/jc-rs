//! Parser for `netstat` command output.
//!
//! Supports Linux (via `netstat_linux.py` logic) and FreeBSD/macOS (via `netstat_freebsd_osx.py`).

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};

pub struct NetstatParser;

static INFO: ParserInfo = ParserInfo {
    name: "netstat",
    argument: "--netstat",
    version: "1.16.0",
    description: "Converts `netstat` command output to JSON",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[Platform::Linux, Platform::Darwin, Platform::FreeBSD],
    tags: &[Tag::Command, Tag::Slurpable],
    magic_commands: &["netstat"],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static NETSTAT_PARSER: NetstatParser = NetstatParser;

inventory::submit! { ParserEntry::new(&NETSTAT_PARSER) }

// ---------------------------------------------------------------------------
// Integer / float field sets (from jc's _process int_list / float_list)
// ---------------------------------------------------------------------------

fn is_int_field(name: &str) -> bool {
    matches!(
        name,
        "recv_q"
            | "send_q"
            | "pid"
            | "refcnt"
            | "inode"
            | "unit"
            | "vendor"
            | "class"
            | "osx_flags"
            | "subcla"
            | "pcbcount"
            | "rcvbuf"
            | "sndbuf"
            | "rxbytes"
            | "txbytes"
            | "route_refs"
            | "use"
            | "mtu"
            | "mss"
            | "window"
            | "irtt"
            | "metric"
            | "ipkts"
            | "ierrs"
            | "opkts"
            | "oerrs"
            | "coll"
            | "rx_ok"
            | "rx_err"
            | "rx_drp"
            | "rx_ovr"
            | "tx_ok"
            | "tx_err"
            | "tx_drp"
            | "tx_ovr"
            | "idrop"
            | "ibytes"
            | "obytes"
            | "r_mbuf"
            | "s_mbuf"
            | "r_clus"
            | "s_clus"
            | "r_hiwa"
            | "s_hiwa"
            | "r_lowa"
            | "s_lowa"
            | "r_bcnt"
            | "s_bcnt"
            | "r_bmax"
            | "s_bmax"
            | "rexmit"
            | "ooorcv"
            | "0_win"
    )
}

fn is_float_field(name: &str) -> bool {
    matches!(
        name,
        "rexmt" | "persist" | "keep" | "2msl" | "delack" | "rcvtime"
    )
}

fn to_int(s: &str) -> Value {
    let t = s.trim();
    if t.is_empty() || t == "-" {
        Value::Null
    } else if let Ok(n) = t.parse::<i64>() {
        Value::Number(n.into())
    } else {
        Value::Null
    }
}

fn to_float(s: &str) -> Value {
    let t = s.trim();
    if t.is_empty() || t == "-" {
        Value::Null
    } else if let Ok(f) = t.parse::<f64>() {
        Value::Number(serde_json::Number::from_f64(f).unwrap_or(0.into()))
    } else {
        Value::Null
    }
}

fn coerce_types(obj: &mut Map<String, Value>) {
    let keys: Vec<String> = obj.keys().cloned().collect();
    for k in &keys {
        if is_int_field(k) {
            let v = obj[k].as_str().map(|s| s.to_string());
            if let Some(s) = v {
                obj.insert(k.clone(), to_int(&s));
            }
        } else if is_float_field(k) {
            let v = obj[k].as_str().map(|s| s.to_string());
            if let Some(s) = v {
                obj.insert(k.clone(), to_float(&s));
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Detection: FreeBSD/OSX vs Linux
// ---------------------------------------------------------------------------

fn is_freebsd_osx(first_line: &str) -> bool {
    matches!(
        first_line,
        "Active Internet connections"
            | "Active Internet connections (including servers)"
            | "Active Multipath Internet connections"
            | "Active LOCAL (UNIX) domain sockets"
            | "Registered kernel control modules"
            | "Active kernel event sockets"
            | "Active kernel control sockets"
            | "Routing tables"
    ) || first_line.starts_with("Name  ")
}

// ---------------------------------------------------------------------------
// Shared address-splitting helpers
// ---------------------------------------------------------------------------

/// Split Linux `addr:port` (handles IPv6 `[::]:22`)
fn split_linux_addr(addr_port: &str) -> (String, String) {
    if addr_port.starts_with('[')
        && let Some(i) = addr_port.rfind(']')
    {
        let addr = addr_port[..=i].to_string();
        let port = if addr_port.len() > i + 1 {
            addr_port[i + 2..].to_string()
        } else {
            String::new()
        };
        return (addr, port);
    }
    if let Some(i) = addr_port.rfind(':') {
        (addr_port[..i].to_string(), addr_port[i + 1..].to_string())
    } else {
        (addr_port.to_string(), String::new())
    }
}

/// Split FreeBSD/OSX `addr.port` (split on last `.`)
fn split_bsd_addr(addr_port: &str) -> (String, String) {
    if let Some(i) = addr_port.rfind('.') {
        (addr_port[..i].to_string(), addr_port[i + 1..].to_string())
    } else {
        (addr_port.to_string(), String::new())
    }
}

fn port_num(port: &str) -> Option<i64> {
    port.parse::<i64>().ok().filter(|&n| n >= 0)
}

/// Split `s` into at most `n+1` whitespace-delimited tokens
/// (mirrors Python's `str.split(maxsplit=n)`).
fn splitn_ws(s: &str, n: usize) -> Vec<String> {
    if n == 0 {
        return vec![s.trim().to_string()];
    }
    let mut result = Vec::new();
    let mut remaining = s.trim_start();
    for _ in 0..n {
        if remaining.is_empty() {
            break;
        }
        let end = remaining
            .find(char::is_whitespace)
            .unwrap_or(remaining.len());
        result.push(remaining[..end].to_string());
        remaining = remaining[end..].trim_start();
    }
    if !remaining.is_empty() {
        result.push(remaining.to_string());
    }
    result
}

// ---------------------------------------------------------------------------
// Linux parser
// ---------------------------------------------------------------------------

fn normalize_linux_net_header(h: &str) -> String {
    h.to_lowercase()
        .replace("local address", "local_address")
        .replace("foreign address", "foreign_address")
        .replace("pid/program name", "program_name")
        .replace("security context", "security_context")
        .replace("i-node", " inode")
        .replace('-', "_")
}

fn normalize_linux_route_header(h: &str) -> String {
    h.to_lowercase()
        .replace("flags", "route_flags")
        .replace("ref", "route_refs")
        .replace('-', "_")
}

fn normalize_linux_iface_header(h: &str) -> String {
    h.to_lowercase().replace('-', "_")
}

fn linux_parse_network(headers: &[String], line: &str) -> Option<Map<String, Value>> {
    let list_of_states = [
        "ESTABLISHED",
        "SYN_SENT",
        "SYN_RECV",
        "FIN_WAIT1",
        "FIN_WAIT2",
        "TIME_WAIT",
        "CLOSED",
        "CLOSE_WAIT",
        "LAST_ACK",
        "LISTEN",
        "CLOSING",
        "UNKNOWN",
        "7",
    ];
    // Python uses \b word-boundary regex. For multi-char tokens (ESTABLISHED etc.)
    // substring match is safe; for "7" we need word-boundary to avoid matching port digits.
    let contains_state = list_of_states.iter().any(|&s| {
        if s == "7" {
            // word-boundary check: "7" must be preceded/followed by non-digit space chars
            line.split_whitespace().any(|w| w == "7")
        } else {
            line.contains(s)
        }
    });
    let split_mod = if contains_state { 1usize } else { 2usize };
    let max_split = headers.len().saturating_sub(split_mod);

    let mut tokens: Vec<Option<String>> = splitn_ws(line, max_split)
        .into_iter()
        .map(|s| Some(s.trim().to_string()))
        .collect();

    // If state column missing (UDP), insert None at index 5
    if tokens.len() == headers.len().saturating_sub(1) && headers.len() > 5 {
        tokens.insert(5, None);
    }

    let mut obj = Map::new();
    for (i, h) in headers.iter().enumerate() {
        let val = tokens
            .get(i)
            .and_then(|v| v.as_deref())
            .unwrap_or("")
            .trim()
            .to_string();
        obj.insert(
            h.clone(),
            if val.is_empty() {
                Value::Null
            } else {
                Value::String(val)
            },
        );
    }
    obj.insert("kind".to_string(), Value::String("network".to_string()));
    Some(obj)
}

fn linux_parse_socket(
    header_text: &str,
    headers: &[String],
    line: &str,
) -> Option<Map<String, Value>> {
    // Column position of "state" in the header string (byte offset)
    let state_col = header_text.find("state").unwrap_or(usize::MAX);

    // Normalise bracket notation: [ ] → ---, [ ACC ] → space-delimited ACC
    let mut entry = line.replace("[ ]", "---");
    entry = entry.replace(['[', ']'], " ");

    // Protect spaces inside program_name column
    let pn_start = header_text.find("program_name");
    let path_start = header_text.find("path");
    if let (Some(ps), Some(pe)) = (pn_start, path_start)
        && pe > ps + 1
    {
        let pn_end = (pe - 1).min(entry.len());
        let ps_clamped = ps.min(entry.len());
        if pn_end > ps_clamped {
            let old_pn = entry[ps_clamped..pn_end].to_string();
            let new_pn = old_pn.replace(' ', "\u{2063}");
            entry = format!("{}{}{}", &entry[..ps_clamped], new_pn, &entry[pn_end..]);
        }
    }

    let mut tokens: Vec<Option<String>> = splitn_ws(&entry, headers.len().saturating_sub(1))
        .into_iter()
        .map(|s| Some(s.trim().to_string()))
        .collect();

    // If state column is blank at the byte offset, insert None at index 4.
    // Mirrors Python: if entry[state_col] in string.whitespace: entry_list.insert(4, None)
    if state_col != usize::MAX {
        let ch = entry.chars().nth(state_col);
        let is_blank = ch.map(|c| c.is_whitespace()).unwrap_or(false);
        if is_blank {
            tokens.insert(4, None);
        }
    }

    let mut obj = Map::new();
    for (i, h) in headers.iter().enumerate() {
        match tokens.get(i) {
            None => {
                // Token missing (out of bounds) — don't include key, mirrors Python's zip()
            }
            Some(None) => {
                // Explicitly inserted None (e.g., state column blank)
                obj.insert(h.clone(), Value::Null);
            }
            Some(Some(s)) => {
                let raw = s.replace('\u{2063}', " ").trim().to_string();
                obj.insert(
                    h.clone(),
                    if raw.is_empty() {
                        Value::Null
                    } else {
                        Value::String(raw)
                    },
                );
            }
        }
    }
    obj.insert("kind".to_string(), Value::String("socket".to_string()));
    Some(obj)
}

fn linux_parse_route(headers: &[String], line: &str) -> Option<Map<String, Value>> {
    let tokens = splitn_ws(line, headers.len().saturating_sub(1));
    if tokens.is_empty() {
        return None;
    }
    let mut obj = Map::new();
    for (i, h) in headers.iter().enumerate() {
        if let Some(val) = tokens.get(i) {
            obj.insert(h.clone(), Value::String(val.trim().to_string()));
        }
        // Missing token (out of bounds) → skip the key entirely (mirrors Python's zip())
    }
    obj.insert("kind".to_string(), Value::String("route".to_string()));
    Some(obj)
}

fn linux_parse_interface(headers: &[String], line: &str) -> Option<Map<String, Value>> {
    let tokens = splitn_ws(line, headers.len().saturating_sub(1));
    if tokens.is_empty() {
        return None;
    }
    let mut obj = Map::new();
    for (i, h) in headers.iter().enumerate() {
        if let Some(val) = tokens.get(i) {
            obj.insert(h.clone(), Value::String(val.trim().to_string()));
        }
    }
    obj.insert("kind".to_string(), Value::String("interface".to_string()));
    Some(obj)
}

fn linux_expand_flags(flags: &str) -> Vec<String> {
    let map = [
        ('U', "UP"),
        ('H', "HOST"),
        ('G', "GATEWAY"),
        ('R', "REINSTATE"),
        ('D', "DYNAMIC"),
        ('M', "MODIFIED"),
        ('A', "ADDRCONF"),
        ('C', "CACHE"),
        ('!', "REJECT"),
    ];
    flags
        .chars()
        .filter_map(|c| {
            map.iter()
                .find(|(fc, _)| *fc == c)
                .map(|(_, n)| n.to_string())
        })
        .collect()
}

fn linux_post_process(entries: Vec<Map<String, Value>>) -> Vec<Map<String, Value>> {
    entries
        .into_iter()
        .map(|mut obj| {
            // Strip trailing whitespace from string values
            let keys: Vec<String> = obj.keys().cloned().collect();
            for k in &keys {
                if let Some(s) = obj[k].as_str() {
                    let t = s.trim_end().to_string();
                    obj.insert(k.clone(), Value::String(t));
                }
            }

            // flags: "---" → null
            if obj.get("flags").and_then(|v| v.as_str()) == Some("---") {
                obj.insert("flags".to_string(), Value::Null);
            }

            // program_name / pid
            if let Some(pn_val) = obj.remove("program_name") {
                let pn = pn_val.as_str().unwrap_or("").trim().to_string();
                if pn.is_empty() || pn == "-" {
                    obj.insert("program_name".to_string(), Value::Null);
                } else if pn.contains('/') {
                    let mut parts = pn.splitn(2, '/');
                    let pid_s = parts.next().unwrap_or("").trim();
                    let name = parts.next().unwrap_or("").trim().to_string();
                    if let Ok(pid) = pid_s.parse::<i64>() {
                        obj.insert("pid".to_string(), Value::Number(pid.into()));
                    }
                    obj.insert("program_name".to_string(), Value::String(name));
                } else {
                    obj.insert("program_name".to_string(), Value::String(pn));
                }
            }

            // local_address → split
            if let Some(la) = obj.remove("local_address")
                && let Some(s) = la.as_str()
            {
                if !s.is_empty() {
                    let (addr, port) = split_linux_addr(s);
                    obj.insert("local_address".to_string(), Value::String(addr));
                    if !port.is_empty() {
                        obj.insert("local_port".to_string(), Value::String(port));
                    }
                } else {
                    obj.insert("local_address".to_string(), Value::Null);
                }
            }

            // foreign_address → split
            if let Some(fa) = obj.remove("foreign_address")
                && let Some(s) = fa.as_str()
            {
                if !s.is_empty() {
                    let (addr, port) = split_linux_addr(s);
                    obj.insert("foreign_address".to_string(), Value::String(addr));
                    if !port.is_empty() {
                        obj.insert("foreign_port".to_string(), Value::String(port));
                    }
                } else {
                    obj.insert("foreign_address".to_string(), Value::Null);
                }
            }

            // transport/network protocol — only for network kind
            if obj.get("kind").and_then(|v| v.as_str()) == Some("network") {
                let proto = obj
                    .get("proto")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let tp = if proto.contains("tcp") {
                    Some("tcp")
                } else if proto.contains("udp") {
                    Some("udp")
                } else {
                    None
                };
                obj.insert(
                    "transport_protocol".to_string(),
                    tp.map(|s| Value::String(s.to_string()))
                        .unwrap_or(Value::Null),
                );
                let np = if proto.contains('6') { "ipv6" } else { "ipv4" };
                obj.insert(
                    "network_protocol".to_string(),
                    Value::String(np.to_string()),
                );
            }

            // route_flags_pretty
            if let Some(flags) = obj
                .get("route_flags")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
            {
                let pretty = linux_expand_flags(&flags);
                obj.insert(
                    "route_flags_pretty".to_string(),
                    Value::Array(pretty.into_iter().map(Value::String).collect()),
                );
            }

            // local_port_num / foreign_port_num
            if let Some(lp) = obj
                .get("local_port")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                && let Some(n) = port_num(&lp)
            {
                obj.insert("local_port_num".to_string(), Value::Number(n.into()));
            }
            if let Some(fp) = obj
                .get("foreign_port")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                && let Some(n) = port_num(&fp)
            {
                obj.insert("foreign_port_num".to_string(), Value::Number(n.into()));
            }

            coerce_types(&mut obj);
            obj
        })
        .collect()
}

fn parse_linux(cleandata: &[&str]) -> Vec<Map<String, Value>> {
    let mut result = Vec::new();
    let mut network = false;
    let mut socket = false;
    let mut routing_table = false;
    let mut interface_table = false;
    let mut headers: Vec<String> = Vec::new();
    let mut header_text = String::new();

    for line in cleandata {
        // Section starters
        if line.starts_with("Active Internet") {
            network = true;
            socket = false;
            routing_table = false;
            interface_table = false;
            continue;
        }
        if line.starts_with("Active UNIX") {
            network = false;
            socket = true;
            routing_table = false;
            interface_table = false;
            continue;
        }
        if line.starts_with("Active Bluetooth") {
            network = false;
            socket = false;
            routing_table = false;
            interface_table = false;
            continue;
        }
        if line.starts_with("Kernel IP routing table") {
            network = false;
            socket = false;
            routing_table = true;
            interface_table = false;
            continue;
        }
        if line.starts_with("Kernel Interface table") {
            network = false;
            socket = false;
            routing_table = false;
            interface_table = true;
            continue;
        }

        // Header lines — do NOT change section flag
        if line.starts_with("Proto") {
            header_text = normalize_linux_net_header(line);
            headers = header_text
                .split_whitespace()
                .map(|s| s.to_string())
                .collect();
            continue;
        }
        if line.starts_with("Destination ") {
            header_text = normalize_linux_route_header(line);
            headers = header_text
                .split_whitespace()
                .map(|s| s.to_string())
                .collect();
            continue;
        }
        if line.starts_with("Iface ") {
            header_text = normalize_linux_iface_header(line);
            headers = header_text
                .split_whitespace()
                .map(|s| s.to_string())
                .collect();
            continue;
        }

        if headers.is_empty() {
            continue;
        }

        // Data lines
        if network {
            if let Some(e) = linux_parse_network(&headers, line) {
                result.push(e);
            }
            continue;
        }
        if socket {
            if let Some(e) = linux_parse_socket(&header_text, &headers, line) {
                result.push(e);
            }
            continue;
        }
        if routing_table {
            if let Some(e) = linux_parse_route(&headers, line) {
                result.push(e);
            }
            continue;
        }
        if interface_table && let Some(e) = linux_parse_interface(&headers, line) {
            result.push(e);
        }
    }

    linux_post_process(result)
}

// ---------------------------------------------------------------------------
// FreeBSD/OSX parser
// ---------------------------------------------------------------------------

fn normalize_bsd_net_header(h: &str) -> String {
    h.to_lowercase()
        .replace("local address", "local_address")
        .replace("foreign address", "foreign_address")
        .replace("(state)", "state")
        .replace("log id", "log")
        .replace("inode", "unix_inode")
        .replace("flags", "unix_flags")
        .replace('-', "_")
}

fn normalize_bsd_route_header(h: &str) -> String {
    h.to_lowercase()
        .replace("flags", "route_flags")
        .replace("refs", "route_refs")
        .replace("netif", "iface")
        .replace('-', "_")
}

fn normalize_bsd_iface_header(h: &str) -> String {
    h.to_lowercase().replace("name", "iface").replace('-', "_")
}

fn normalize_bsd_socket_header(h: &str) -> String {
    h.to_lowercase()
        .replace("inode", "unix_inode")
        .replace("flags", "unix_flags")
        .replace('-', "_")
}

fn bsd_parse_item(
    headers: &[String],
    line: &str,
    kind: &str,
    has_state_col: bool,
    has_0win_col: bool,
    has_socket_col: bool,
) -> Option<Map<String, Value>> {
    let mut tokens = splitn_ws(line, headers.len().saturating_sub(1));
    let first = tokens.first().cloned().unwrap_or_default();

    if kind == "network" {
        // is_udp_first: proto column is first token (standard layout)
        let is_udp_first = first.to_lowercase().starts_with("udp");
        // is_udp_any: proto may not be first (socket-address layout), check all tokens
        let is_udp_any = is_udp_first || tokens.iter().any(|t| t.to_lowercase().contains("udp"));

        // netstat -aT style: rexmit/ooorcv/0_win columns missing for UDP
        if has_0win_col && is_udp_first {
            tokens.insert(1, String::new());
            tokens.insert(1, String::new());
            tokens.insert(1, String::new());
        } else if has_state_col && is_udp_first && !has_socket_col {
            // Standard: state column missing for UDP — insert at position 5
            if tokens.len() < headers.len() {
                tokens.insert(5, String::new());
            }
        }
        // socket column present (netstat -An/-Aa style): UDP missing state at position 7
        // Python: if 'socket' in headers and 'udp' in str(entry): entry.insert(7, None)
        if has_socket_col && is_udp_any && !has_0win_col && tokens.len() < headers.len() {
            tokens.insert(7, String::new());
        }
    }
    // Interface with missing address column (OSX netstat -i)
    // Python: if kind == 'interface' and len(entry) == 8: entry.insert(3, None)
    if kind == "interface" && tokens.len() == 8 {
        tokens.insert(3, String::new());
    }

    let mut obj = Map::new();
    for (i, h) in headers.iter().enumerate() {
        // Skip missing tokens — mirrors Python's zip() which stops at shortest
        let Some(val_raw) = tokens.get(i) else {
            continue;
        };
        let val = val_raw.trim().to_string();
        obj.insert(
            h.clone(),
            if val.is_empty() {
                Value::Null
            } else {
                Value::String(val)
            },
        );
    }
    obj.insert("kind".to_string(), Value::String(kind.to_string()));
    Some(obj)
}

fn bsd_transport_protocol(proto: &str) -> Option<&'static str> {
    if proto == "udp46" {
        Some("udp")
    } else if proto.starts_with("icm") {
        Some("icmp")
    } else if proto.starts_with("tcp") || proto.starts_with("kctl") || proto.starts_with("kevt") {
        Some("tcp")
    } else if proto.starts_with("udp") {
        Some("udp")
    } else if proto.len() > 1 {
        // jc does proto[:-1] for things like tcp4/tcp6/udp4/udp6
        // We handle the common cases above; fallback to stripping last char
        None
    } else {
        None
    }
}

fn bsd_expand_flags(flags: &str) -> Vec<String> {
    let map: &[(&str, &str)] = &[
        ("1", "PROTO1"),
        ("2", "PROTO2"),
        ("3", "PROTO3"),
        ("B", "BLACKHOLE"),
        ("b", "BROADCAST"),
        ("C", "CLONING"),
        ("c", "PRCLONING"),
        ("D", "DYNAMIC"),
        ("G", "GATEWAY"),
        ("H", "HOST"),
        ("I", "IFSCOPE"),
        ("i", "IFREF"),
        ("L", "LLINFO"),
        ("M", "MODIFIED"),
        ("m", "MULTICAST"),
        ("R", "REJECT"),
        ("r", "ROUTER"),
        ("S", "STATIC"),
        ("U", "UP"),
        ("W", "WASCLONED"),
        ("X", "XRESOLVE"),
        ("Y", "PROXY"),
    ];
    flags
        .chars()
        .filter_map(|c| {
            let s = c.to_string();
            map.iter()
                .find(|(fc, _)| *fc == s.as_str())
                .map(|(_, n)| n.to_string())
        })
        .collect()
}

fn bsd_post_process(entries: Vec<Map<String, Value>>) -> Vec<Map<String, Value>> {
    entries
        .into_iter()
        .map(|mut obj| {
            // Strip whitespace from string values
            let keys: Vec<String> = obj.keys().cloned().collect();
            for k in &keys {
                if let Some(s) = obj[k].as_str() {
                    let t = s.trim().to_string();
                    obj.insert(
                        k.clone(),
                        if t.is_empty() {
                            Value::Null
                        } else {
                            Value::String(t)
                        },
                    );
                }
            }

            if obj.get("kind").and_then(|v| v.as_str()) == Some("network") {
                // local_address → split on last '.'
                if let Some(la) = obj.remove("local_address")
                    && let Some(s) = la.as_str()
                {
                    if !s.is_empty() {
                        let (addr, port) = split_bsd_addr(s);
                        obj.insert("local_address".to_string(), Value::String(addr));
                        if !port.is_empty() {
                            obj.insert("local_port".to_string(), Value::String(port));
                        }
                    } else {
                        obj.insert("local_address".to_string(), Value::Null);
                    }
                }

                // foreign_address → split on last '.'
                if let Some(fa) = obj.remove("foreign_address")
                    && let Some(s) = fa.as_str()
                {
                    if !s.is_empty() {
                        let (addr, port) = split_bsd_addr(s);
                        obj.insert("foreign_address".to_string(), Value::String(addr));
                        if !port.is_empty() {
                            obj.insert("foreign_port".to_string(), Value::String(port));
                        }
                    } else {
                        obj.insert("foreign_address".to_string(), Value::Null);
                    }
                }

                // transport/network protocol
                let proto = obj
                    .get("proto")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let tp = bsd_transport_protocol(&proto);
                obj.insert(
                    "transport_protocol".to_string(),
                    tp.map(|s| Value::String(s.to_string()))
                        .unwrap_or(Value::Null),
                );
                let np = if proto.contains('6') { "ipv6" } else { "ipv4" };
                obj.insert(
                    "network_protocol".to_string(),
                    Value::String(np.to_string()),
                );
            }

            // route_flags_pretty
            if let Some(flags) = obj
                .get("route_flags")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
            {
                let pretty = bsd_expand_flags(&flags);
                obj.insert(
                    "route_flags_pretty".to_string(),
                    Value::Array(pretty.into_iter().map(Value::String).collect()),
                );
            }

            // local_port_num / foreign_port_num
            if let Some(lp) = obj
                .get("local_port")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                && let Some(n) = port_num(&lp)
            {
                obj.insert("local_port_num".to_string(), Value::Number(n.into()));
            }
            if let Some(fp) = obj
                .get("foreign_port")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                && let Some(n) = port_num(&fp)
            {
                obj.insert("foreign_port_num".to_string(), Value::Number(n.into()));
            }

            coerce_types(&mut obj);
            obj
        })
        .collect()
}

fn parse_freebsd_osx(cleandata: &[&str]) -> Vec<Map<String, Value>> {
    let mut result = Vec::new();
    let mut network = false;
    let mut socket = false;
    let mut reg_kernel_control = false;
    let mut active_kernel_event = false;
    let mut active_kernel_control = false;
    let mut routing_table = false;
    let mut interface_table = false;

    let mut headers: Vec<String> = Vec::new();
    let mut has_state_col = false;
    let mut has_0win_col = false;
    let mut has_socket_col = false;

    for line in cleandata {
        // Section starters
        if line.starts_with("Active Internet") || line.starts_with("Active Multipath Internet") {
            network = true;
            socket = false;
            reg_kernel_control = false;
            active_kernel_event = false;
            active_kernel_control = false;
            routing_table = false;
            interface_table = false;
            headers.clear();
            continue;
        }
        if line.starts_with("Active LOCAL (UNIX) domain sockets")
            || line.starts_with("Active UNIX domain sockets")
        {
            network = false;
            socket = true;
            reg_kernel_control = false;
            active_kernel_event = false;
            active_kernel_control = false;
            routing_table = false;
            interface_table = false;
            headers.clear();
            continue;
        }
        if line.starts_with("Registered kernel control modules") {
            network = false;
            socket = false;
            reg_kernel_control = true;
            active_kernel_event = false;
            active_kernel_control = false;
            routing_table = false;
            interface_table = false;
            headers.clear();
            continue;
        }
        if line.starts_with("Active kernel event sockets") {
            network = false;
            socket = false;
            reg_kernel_control = false;
            active_kernel_event = true;
            active_kernel_control = false;
            routing_table = false;
            interface_table = false;
            headers.clear();
            continue;
        }
        if line.starts_with("Active kernel control sockets") {
            network = false;
            socket = false;
            reg_kernel_control = false;
            active_kernel_event = false;
            active_kernel_control = true;
            routing_table = false;
            interface_table = false;
            headers.clear();
            continue;
        }
        if line.starts_with("Routing tables") {
            network = false;
            socket = false;
            reg_kernel_control = false;
            active_kernel_event = false;
            active_kernel_control = false;
            routing_table = true;
            interface_table = false;
            headers.clear();
            continue;
        }
        // Internet: / Internet6: sub-headers — reset headers for next sub-table
        if routing_table && (line.starts_with("Internet:") || line.starts_with("Internet6:")) {
            headers.clear();
            continue;
        }
        // Interface table: "Name  " is both section start and header
        if line.starts_with("Name  ") {
            network = false;
            socket = false;
            reg_kernel_control = false;
            active_kernel_event = false;
            active_kernel_control = false;
            routing_table = false;
            interface_table = true;
            let ht = normalize_bsd_iface_header(line);
            headers = ht.split_whitespace().map(|s| s.to_string()).collect();
            continue;
        }

        // Header lines
        if network
            && (line.starts_with("Socket ")
                || line.starts_with("Proto ")
                || line.starts_with("Tcpcb "))
        {
            let ht = normalize_bsd_net_header(line);
            headers = ht.split_whitespace().map(|s| s.to_string()).collect();
            has_state_col = headers.contains(&"state".to_string());
            has_0win_col = headers.contains(&"0_win".to_string());
            has_socket_col = headers.contains(&"socket".to_string());
            continue;
        }
        if socket && line.starts_with("Address ") {
            let ht = normalize_bsd_socket_header(line);
            headers = ht.split_whitespace().map(|s| s.to_string()).collect();
            continue;
        }
        if reg_kernel_control && (line.starts_with("id ") || line.starts_with("kctlref ")) {
            let ht = normalize_bsd_net_header(line);
            headers = ht.split_whitespace().map(|s| s.to_string()).collect();
            continue;
        }
        if (active_kernel_event || active_kernel_control)
            && (line.starts_with("Proto ") || line.starts_with("             pcb "))
        {
            let ht = normalize_bsd_net_header(line);
            headers = ht.split_whitespace().map(|s| s.to_string()).collect();
            continue;
        }
        if routing_table && line.starts_with("Destination ") {
            let ht = normalize_bsd_route_header(line);
            headers = ht.split_whitespace().map(|s| s.to_string()).collect();
            continue;
        }

        if headers.is_empty() {
            continue;
        }

        // Data lines
        if network {
            if let Some(e) = bsd_parse_item(
                &headers,
                line,
                "network",
                has_state_col,
                has_0win_col,
                has_socket_col,
            ) {
                result.push(e);
            }
            continue;
        }
        if socket {
            if let Some(e) = bsd_parse_item(&headers, line, "socket", false, false, false) {
                result.push(e);
            }
            continue;
        }
        if reg_kernel_control {
            if let Some(e) = bsd_parse_item(
                &headers,
                line,
                "Registered kernel control module",
                false,
                false,
                false,
            ) {
                result.push(e);
            }
            continue;
        }
        if active_kernel_event {
            if let Some(e) = bsd_parse_item(
                &headers,
                line,
                "Active kernel event socket",
                false,
                false,
                false,
            ) {
                result.push(e);
            }
            continue;
        }
        if active_kernel_control {
            if let Some(e) = bsd_parse_item(
                &headers,
                line,
                "Active kernel control socket",
                false,
                false,
                false,
            ) {
                result.push(e);
            }
            continue;
        }
        if routing_table {
            if let Some(e) = bsd_parse_item(&headers, line, "route", false, false, false) {
                result.push(e);
            }
            continue;
        }
        if interface_table
            && let Some(e) = bsd_parse_item(&headers, line, "interface", false, false, false)
        {
            result.push(e);
        }
    }

    bsd_post_process(result)
}

// ---------------------------------------------------------------------------
// Parser trait impl
// ---------------------------------------------------------------------------

impl Parser for NetstatParser {
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

        let result = if is_freebsd_osx(cleandata[0]) {
            parse_freebsd_osx(&cleandata)
        } else {
            parse_linux(&cleandata)
        };

        Ok(ParseOutput::Array(result))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jc_rs_core::traits::Parser;

    #[test]
    fn test_netstat_centos_golden() {
        let input = include_str!("../../../../tests/fixtures/centos-7.7/netstat.out");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/centos-7.7/netstat.json"
        ))
        .unwrap();
        let result = NetstatParser.parse(input, false).unwrap();
        let actual = serde_json::to_value(result).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_netstat_centos_r_golden() {
        let input = include_str!("../../../../tests/fixtures/centos-7.7/netstat-r.out");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/centos-7.7/netstat-r.json"
        ))
        .unwrap();
        let result = NetstatParser.parse(input, false).unwrap();
        let actual = serde_json::to_value(result).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_netstat_empty() {
        let result = NetstatParser.parse("", false).unwrap();
        assert!(matches!(result, ParseOutput::Array(v) if v.is_empty()));
    }

    #[test]
    fn test_netstat_registered() {
        assert!(jc_rs_core::registry::find_parser("netstat").is_some());
    }
}
