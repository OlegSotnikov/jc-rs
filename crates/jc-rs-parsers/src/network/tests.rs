//! Tests for network command parsers.

use jc_rs_core::registry::find_parser;
use jc_rs_core::types::ParseOutput;

fn get_fixture(rel_path: &str) -> String {
    // rel_path is relative to the fixtures dir, e.g. "centos-7.7/arp.out"
    // CARGO_MANIFEST_DIR: .../jc-rs/crates/jc-rs-parsers
    // Fixtures are at:    .../jc-rs/tests/fixtures/
    let env_manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default();
    let candidates = [
        format!("{env_manifest}/../../tests/fixtures/{rel_path}"),
        format!("{env_manifest}/../../../tests/fixtures/{rel_path}"),
        format!("tests/fixtures/{rel_path}"),
        format!("../../tests/fixtures/{rel_path}"),
    ];
    for path in &candidates {
        if let Ok(content) = std::fs::read_to_string(path) {
            return content;
        }
    }
    let tried: Vec<String> = candidates
        .iter()
        .map(|p| format!("  {} (exists={})", p, std::path::Path::new(p).exists()))
        .collect();
    panic!(
        "fixture not found: {rel_path}\nTried:\n{}",
        tried.join("\n")
    )
}

// ─── ARP ────────────────────────────────────────────────────────────────────

#[test]
fn test_arp_registered() {
    assert!(find_parser("arp").is_some());
}

#[test]
fn test_arp_linux_table() {
    let input = get_fixture("centos-7.7/arp.out");
    let parser = find_parser("arp").unwrap();
    let result = parser.parse(&input, true).unwrap();
    let arr = match result {
        ParseOutput::Array(v) => v,
        _ => panic!("expected array"),
    };
    assert!(!arr.is_empty());
    let first = &arr[0];
    assert!(first.contains_key("address"));
    assert!(first.contains_key("hwtype"));
    assert!(first.contains_key("hwaddress"));
    assert!(first.contains_key("flags_mask"));
    assert!(first.contains_key("iface"));
}

#[test]
fn test_arp_bsd_a_style() {
    let input = get_fixture("osx-10.14.6/arp-a.out");
    let parser = find_parser("arp").unwrap();
    let result = parser.parse(&input, true).unwrap();
    let arr = match result {
        ParseOutput::Array(v) => v,
        _ => panic!("expected array"),
    };
    assert!(!arr.is_empty());
    let first = &arr[0];
    assert!(first.contains_key("name"));
    assert!(first.contains_key("address"));
    assert!(first.contains_key("hwtype"));
    assert!(first.contains_key("hwaddress"));
    assert!(first.contains_key("iface"));
    assert!(first.contains_key("permanent"));
}

#[test]
fn test_arp_empty() {
    let parser = find_parser("arp").unwrap();
    let result = parser.parse("", true).unwrap();
    let arr = match result {
        ParseOutput::Array(v) => v,
        _ => panic!("expected array"),
    };
    assert!(arr.is_empty());
}

// ─── DIG ────────────────────────────────────────────────────────────────────

#[test]
fn test_dig_registered() {
    assert!(find_parser("dig").is_some());
}

#[test]
fn test_dig_additional_fixture() {
    let input = get_fixture("generic/dig-additional.out");
    let parser = find_parser("dig").unwrap();
    let result = parser.parse(&input, true).unwrap();
    let arr = match result {
        ParseOutput::Array(v) => v,
        _ => panic!("expected array"),
    };
    assert!(!arr.is_empty(), "dig should produce at least one result");
    let entry = &arr[0];
    assert!(entry.contains_key("id"), "missing id");
    assert!(entry.contains_key("status"), "missing status");
    assert!(entry.contains_key("flags"), "missing flags");
    assert!(entry.contains_key("query_time"), "missing query_time");
}

#[test]
fn test_dig_noall_answer() {
    let input = get_fixture("osx-10.14.6/dig-noall-answer.out");
    let parser = find_parser("dig").unwrap();
    let result = parser.parse(&input, true).unwrap();
    let arr = match result {
        ParseOutput::Array(v) => v,
        _ => panic!("expected array"),
    };
    assert!(!arr.is_empty());
    let entry = &arr[0];
    assert!(entry.contains_key("answer"), "should have answer section");
}

#[test]
fn test_dig_edns() {
    let input = get_fixture("generic/dig-edns.out");
    let parser = find_parser("dig").unwrap();
    let result = parser.parse(&input, true).unwrap();
    let arr = match result {
        ParseOutput::Array(v) => v,
        _ => panic!("expected array"),
    };
    assert!(!arr.is_empty());
    let entry = &arr[0];
    assert!(
        entry.contains_key("opt_pseudosection"),
        "missing opt_pseudosection"
    );
}

#[test]
fn test_dig_empty() {
    let parser = find_parser("dig").unwrap();
    let result = parser.parse("", true).unwrap();
    let arr = match result {
        ParseOutput::Array(v) => v,
        _ => panic!("expected array"),
    };
    assert!(arr.is_empty());
}

// ─── IFCONFIG ───────────────────────────────────────────────────────────────

#[test]
fn test_ifconfig_registered() {
    assert!(find_parser("ifconfig").is_some());
}

#[test]
fn test_ifconfig_linux() {
    let input = get_fixture("centos-7.7/ifconfig.out");
    let parser = find_parser("ifconfig").unwrap();
    let result = parser.parse(&input, true).unwrap();
    let arr = match result {
        ParseOutput::Array(v) => v,
        _ => panic!("expected array"),
    };
    assert!(!arr.is_empty(), "ifconfig should produce interfaces");
    let first = &arr[0];
    assert!(first.contains_key("name"), "missing name");
    assert!(first.contains_key("mtu"), "missing mtu");
}

#[test]
fn test_ifconfig_macos() {
    let input = get_fixture("osx-10.14.6/ifconfig.out");
    let parser = find_parser("ifconfig").unwrap();
    let result = parser.parse(&input, true).unwrap();
    let arr = match result {
        ParseOutput::Array(v) => v,
        _ => panic!("expected array"),
    };
    assert!(!arr.is_empty(), "ifconfig macOS should produce interfaces");
}

#[test]
fn test_ifconfig_empty() {
    let parser = find_parser("ifconfig").unwrap();
    let result = parser.parse("", true).unwrap();
    let arr = match result {
        ParseOutput::Array(v) => v,
        _ => panic!("expected array"),
    };
    assert!(arr.is_empty());
}

// ─── NETSTAT ────────────────────────────────────────────────────────────────

#[test]
fn test_netstat_registered() {
    assert!(find_parser("netstat").is_some());
}

#[test]
fn test_netstat_linux() {
    let input = get_fixture("centos-7.7/netstat.out");
    let parser = find_parser("netstat").unwrap();
    let result = parser.parse(&input, true).unwrap();
    let arr = match result {
        ParseOutput::Array(v) => v,
        _ => panic!("expected array"),
    };
    assert!(!arr.is_empty(), "netstat should produce results");
}

#[test]
fn test_netstat_route() {
    let input = get_fixture("centos-7.7/netstat-r.out");
    let parser = find_parser("netstat").unwrap();
    let result = parser.parse(&input, true).unwrap();
    let arr = match result {
        ParseOutput::Array(v) => v,
        _ => panic!("expected array"),
    };
    assert!(!arr.is_empty(), "netstat -r should produce routes");
}

#[test]
fn test_netstat_empty() {
    let parser = find_parser("netstat").unwrap();
    let result = parser.parse("", true).unwrap();
    let arr = match result {
        ParseOutput::Array(v) => v,
        _ => panic!("expected array"),
    };
    assert!(arr.is_empty());
}

// ─── PING ───────────────────────────────────────────────────────────────────

#[test]
fn test_ping_registered() {
    assert!(find_parser("ping").is_some());
}

#[test]
fn test_ping_linux() {
    let input = get_fixture("osx-10.14.6/ping-hostname.out");
    let parser = find_parser("ping").unwrap();
    let result = parser.parse(&input, true).unwrap();
    let obj = match result {
        ParseOutput::Object(m) => m,
        _ => panic!("expected object"),
    };
    assert!(obj.contains_key("responses"), "missing responses");
    assert!(
        obj.contains_key("destination_ip") || obj.contains_key("destination"),
        "missing destination"
    );
}

#[test]
fn test_ping_empty() {
    let parser = find_parser("ping").unwrap();
    let result = parser.parse("", true).unwrap();
    let obj = match result {
        ParseOutput::Object(m) => m,
        _ => panic!("expected object"),
    };
    assert!(obj.is_empty());
}

// ─── SS ─────────────────────────────────────────────────────────────────────

#[test]
fn test_ss_registered() {
    assert!(find_parser("ss").is_some());
}

#[test]
fn test_ss_wide() {
    let input = get_fixture("generic/ss-wide.out");
    let parser = find_parser("ss").unwrap();
    let result = parser.parse(&input, true).unwrap();
    let arr = match result {
        ParseOutput::Array(v) => v,
        _ => panic!("expected array"),
    };
    assert!(!arr.is_empty(), "ss should produce results");
}

#[test]
fn test_ss_empty() {
    let parser = find_parser("ss").unwrap();
    let result = parser.parse("", true).unwrap();
    let arr = match result {
        ParseOutput::Array(v) => v,
        _ => panic!("expected array"),
    };
    assert!(arr.is_empty());
}

// ─── IP ADDRESS ─────────────────────────────────────────────────────────────

#[test]
fn test_ip_address_registered() {
    assert!(find_parser("ip_address").is_some());
}

#[test]
fn test_ip_address_cidr() {
    let parser = find_parser("ip_address").unwrap();
    let result = parser.parse("192.168.2.10/24", true).unwrap();
    let obj = match result {
        ParseOutput::Object(m) => m,
        _ => panic!("expected object"),
    };
    assert!(!obj.is_empty(), "ip_address should parse CIDR");
    assert_eq!(obj["version"], serde_json::Value::Number(4i64.into()));
    assert_eq!(obj["cidr_netmask"], serde_json::Value::Number(24i64.into()));
    assert_eq!(
        obj["netmask"],
        serde_json::Value::String("255.255.255.0".to_string())
    );
    assert_eq!(
        obj["network"],
        serde_json::Value::String("192.168.2.0".to_string())
    );
    assert_eq!(
        obj["broadcast"],
        serde_json::Value::String("192.168.2.255".to_string())
    );
}

#[test]
fn test_ip_address_plain() {
    let parser = find_parser("ip_address").unwrap();
    let result = parser.parse("192.168.1.1", true).unwrap();
    let obj = match result {
        ParseOutput::Object(m) => m,
        _ => panic!("expected object"),
    };
    assert!(!obj.is_empty());
    assert_eq!(obj["version"], serde_json::Value::Number(4i64.into()));
}

#[test]
fn test_ip_address_empty() {
    // The ip_address parser returns Err on empty input (jc behavior)
    let parser = find_parser("ip_address").unwrap();
    assert!(parser.parse("", true).is_err());
}

// ─── IP ROUTE ───────────────────────────────────────────────────────────────

#[test]
fn test_ip_route_registered() {
    assert!(find_parser("ip_route").is_some());
}

#[test]
fn test_ip_route_centos() {
    let input = get_fixture("centos-7.7/ip_route.out");
    let parser = find_parser("ip_route").unwrap();
    let result = parser.parse(&input, true).unwrap();
    let arr = match result {
        ParseOutput::Array(v) => v,
        _ => panic!("expected array"),
    };
    assert_eq!(arr.len(), 4, "expected 4 routes");
    // Check first route (default)
    assert_eq!(
        arr[0]["ip"],
        serde_json::Value::String("default".to_string())
    );
    assert_eq!(
        arr[0]["via"],
        serde_json::Value::String("10.0.2.2".to_string())
    );
    assert_eq!(
        arr[0]["dev"],
        serde_json::Value::String("enp0s3".to_string())
    );
    assert_eq!(arr[0]["metric"], serde_json::Value::Number(100i64.into()));
    // Check linkdown
    assert_eq!(
        arr[3]["status"],
        serde_json::Value::String("linkdown".to_string())
    );
}

#[test]
fn test_ip_route_empty() {
    let parser = find_parser("ip_route").unwrap();
    let result = parser.parse("", true).unwrap();
    let arr = match result {
        ParseOutput::Array(v) => v,
        _ => panic!("expected array"),
    };
    assert!(arr.is_empty());
}

// ─── HOST ───────────────────────────────────────────────────────────────────

#[test]
fn test_host_registered() {
    assert!(find_parser("host").is_some());
}

#[test]
fn test_host_google() {
    let input = get_fixture("generic/host-google.out");
    let parser = find_parser("host").unwrap();
    let result = parser.parse(&input, true).unwrap();
    let arr = match result {
        ParseOutput::Array(v) => v,
        _ => panic!("expected array"),
    };
    assert!(!arr.is_empty());
    let entry = &arr[0];
    assert_eq!(
        entry["hostname"],
        serde_json::Value::String("google.com".to_string())
    );
    assert!(entry.contains_key("address"), "missing address field");
    assert!(entry.contains_key("v6-address"), "missing v6-address field");
    assert!(entry.contains_key("mail"), "missing mail field");
}

#[test]
fn test_host_empty() {
    let parser = find_parser("host").unwrap();
    let result = parser.parse("", true).unwrap();
    let arr = match result {
        ParseOutput::Array(v) => v,
        _ => panic!("expected array"),
    };
    assert!(arr.is_empty());
}

// ─── TRACEROUTE ─────────────────────────────────────────────────────────────

#[test]
fn test_traceroute_registered() {
    assert!(find_parser("traceroute").is_some());
}

#[test]
fn test_traceroute_ipv4() {
    let input = get_fixture("generic/traceroute-n-ipv4.out");
    let parser = find_parser("traceroute").unwrap();
    let result = parser.parse(&input, true).unwrap();
    let obj = match result {
        ParseOutput::Object(m) => m,
        _ => panic!("expected object"),
    };
    assert!(!obj.is_empty());
    assert_eq!(
        obj["destination_ip"],
        serde_json::Value::String("199.58.80.40".to_string())
    );
    assert_eq!(
        obj["destination_name"],
        serde_json::Value::String("www.koumbit.org".to_string())
    );
    assert_eq!(obj["max_hops"], serde_json::Value::Number(30i64.into()));
    assert_eq!(obj["data_bytes"], serde_json::Value::Number(60i64.into()));
    let hops = obj["hops"].as_array().unwrap();
    assert_eq!(hops.len(), 8, "expected 8 hops");
    // First hop has 3 probes
    let hop1 = hops[0].as_object().unwrap();
    assert_eq!(hop1["hop"], serde_json::Value::Number(1i64.into()));
    let probes = hop1["probes"].as_array().unwrap();
    assert_eq!(probes.len(), 3, "hop 1 should have 3 probes");
    // Check first probe IP
    assert_eq!(
        probes[0]["ip"],
        serde_json::Value::String("192.168.2.1".to_string())
    );
}

#[test]
fn test_traceroute_empty() {
    let parser = find_parser("traceroute").unwrap();
    let result = parser.parse("", true).unwrap();
    let obj = match result {
        ParseOutput::Object(m) => m,
        _ => panic!("expected object"),
    };
    assert!(obj.is_empty());
}

// ─── ROUTE ──────────────────────────────────────────────────────────────────

#[test]
fn test_route_registered() {
    assert!(find_parser("route").is_some());
}

#[test]
fn test_route_linux() {
    let input = get_fixture("centos-7.7/route.out");
    let parser = find_parser("route").unwrap();
    let result = parser.parse(&input, true).unwrap();
    let arr = match result {
        ParseOutput::Array(v) => v,
        _ => panic!("expected array"),
    };
    assert!(!arr.is_empty(), "route should produce entries");
    let first = &arr[0];
    assert!(first.contains_key("destination"), "missing destination");
    assert!(first.contains_key("gateway"), "missing gateway");
    assert!(first.contains_key("flags"), "missing flags");
}

#[test]
fn test_route_linux_ipv6() {
    let input = get_fixture("centos-7.7/route-6-n.out");
    let parser = find_parser("route").unwrap();
    let result = parser.parse(&input, true).unwrap();
    let arr = match result {
        ParseOutput::Array(v) => v,
        _ => panic!("expected array"),
    };
    assert!(!arr.is_empty(), "route -6 -n should produce entries");
}

#[test]
fn test_route_empty() {
    let parser = find_parser("route").unwrap();
    let result = parser.parse("", true).unwrap();
    let arr = match result {
        ParseOutput::Array(v) => v,
        _ => panic!("expected array"),
    };
    assert!(arr.is_empty());
}

// ─── ROUTE PRINT ────────────────────────────────────────────────────────────

#[test]
fn test_route_print_registered() {
    assert!(find_parser("route_print").is_some());
}

#[test]
fn test_route_print_inline() {
    let input = "===========================================================================\nInterface List\n  6...00 50 56 c0 00 08 ......vmxnet3 Ethernet Adapter\n===========================================================================\n\nIPv4 Route Table\n===========================================================================\nActive Routes:\nNetwork Destination        Netmask          Gateway       Interface  Metric\n          0.0.0.0          0.0.0.0      192.168.1.1   192.168.1.100     25\n===========================================================================\n";
    let parser = find_parser("route_print").unwrap();
    let result = parser.parse(input, true).unwrap();
    let obj = match result {
        ParseOutput::Object(m) => m,
        _ => panic!("expected object"),
    };
    assert!(obj.contains_key("interface_list"), "missing interface_list");
    assert!(
        obj.contains_key("ipv4_route_table"),
        "missing ipv4_route_table"
    );
}

#[test]
fn test_route_print_empty() {
    let parser = find_parser("route_print").unwrap();
    let result = parser.parse("", true).unwrap();
    let obj = match result {
        ParseOutput::Object(m) => m,
        _ => panic!("expected object"),
    };
    assert!(obj.is_empty());
}

// ─── TRACEPATH ──────────────────────────────────────────────────────────────

#[test]
fn test_tracepath_registered() {
    assert!(find_parser("tracepath").is_some());
}

#[test]
fn test_tracepath_centos() {
    let input = get_fixture("centos-7.7/tracepath.out");
    let parser = find_parser("tracepath").unwrap();
    let result = parser.parse(&input, true).unwrap();
    let obj = match result {
        ParseOutput::Object(m) => m,
        _ => panic!("expected object"),
    };
    assert!(obj.contains_key("hops"), "missing hops");
    let hops = obj["hops"].as_array().unwrap();
    assert!(!hops.is_empty(), "hops should not be empty");
}

#[test]
fn test_tracepath_empty() {
    let parser = find_parser("tracepath").unwrap();
    let result = parser.parse("", true).unwrap();
    let obj = match result {
        ParseOutput::Object(m) => m,
        _ => panic!("expected object"),
    };
    assert!(obj.is_empty());
}

// ─── PING_S ─────────────────────────────────────────────────────────────────

#[test]
fn test_ping_s_registered() {
    assert!(find_parser("ping_s").is_some());
}

#[test]
fn test_ping_s_linux() {
    let input = get_fixture("centos-7.7/ping-ip-O.out");
    let parser = find_parser("ping_s").unwrap();
    let result = parser.parse(&input, true).unwrap();
    let arr = match result {
        ParseOutput::Array(v) => v,
        _ => panic!("expected array"),
    };
    assert!(!arr.is_empty(), "ping_s should produce records");
    let has_summary = arr
        .iter()
        .any(|r| r.get("type").and_then(|v| v.as_str()) == Some("summary"));
    assert!(has_summary, "ping_s should produce a summary record");
}

#[test]
fn test_ping_s_empty() {
    let parser = find_parser("ping_s").unwrap();
    let result = parser.parse("", true).unwrap();
    let arr = match result {
        ParseOutput::Array(v) => v,
        _ => panic!("expected array"),
    };
    assert!(arr.is_empty());
}

// ─── TRACEROUTE_S ───────────────────────────────────────────────────────────

#[test]
fn test_traceroute_s_registered() {
    assert!(find_parser("traceroute_s").is_some());
}

#[test]
fn test_traceroute_s_ipv4() {
    let input = get_fixture("generic/traceroute-n-ipv4.out");
    let parser = find_parser("traceroute_s").unwrap();
    let result = parser.parse(&input, true).unwrap();
    let arr = match result {
        ParseOutput::Array(v) => v,
        _ => panic!("expected array"),
    };
    assert!(!arr.is_empty(), "traceroute_s should produce records");
    let has_hop = arr
        .iter()
        .any(|r| r.get("type").and_then(|v| v.as_str()) == Some("hop"));
    assert!(has_hop, "should have hop records");
}

#[test]
fn test_traceroute_s_empty() {
    let parser = find_parser("traceroute_s").unwrap();
    let result = parser.parse("", true).unwrap();
    let arr = match result {
        ParseOutput::Array(v) => v,
        _ => panic!("expected array"),
    };
    assert!(arr.is_empty());
}

// ─── RSYNC ──────────────────────────────────────────────────────────────────

#[test]
fn test_rsync_registered() {
    assert!(find_parser("rsync").is_some());
}

#[test]
fn test_rsync_i() {
    let input = get_fixture("generic/rsync-i.out");
    let parser = find_parser("rsync").unwrap();
    let result = parser.parse(&input, true).unwrap();
    let arr = match result {
        ParseOutput::Array(v) => v,
        _ => panic!("expected array"),
    };
    assert!(!arr.is_empty(), "rsync should produce output");
}

#[test]
fn test_rsync_empty() {
    let parser = find_parser("rsync").unwrap();
    let result = parser.parse("", true).unwrap();
    let arr = match result {
        ParseOutput::Array(v) => v,
        _ => panic!("expected array"),
    };
    assert!(arr.is_empty());
}

// ─── RSYNC_S ────────────────────────────────────────────────────────────────

#[test]
fn test_rsync_s_registered() {
    assert!(find_parser("rsync_s").is_some());
}

#[test]
fn test_rsync_s_i() {
    let input = get_fixture("generic/rsync-i.out");
    let parser = find_parser("rsync_s").unwrap();
    let result = parser.parse(&input, true).unwrap();
    let arr = match result {
        ParseOutput::Array(v) => v,
        _ => panic!("expected array"),
    };
    assert!(!arr.is_empty(), "rsync_s should produce records");
    let has_file = arr
        .iter()
        .any(|r| r.get("type").and_then(|v| v.as_str()) == Some("file"));
    assert!(has_file, "rsync_s should have file records");
}

#[test]
fn test_rsync_s_empty() {
    let parser = find_parser("rsync_s").unwrap();
    let result = parser.parse("", true).unwrap();
    let arr = match result {
        ParseOutput::Array(v) => v,
        _ => panic!("expected array"),
    };
    assert!(arr.is_empty());
}

// ─── NMCLI ──────────────────────────────────────────────────────────────────

#[test]
fn test_nmcli_registered() {
    assert!(find_parser("nmcli").is_some());
}

#[test]
fn test_nmcli_device_show() {
    let input = get_fixture("centos-7.7/nmcli-device-show.out");
    let parser = find_parser("nmcli").unwrap();
    let result = parser.parse(&input, true).unwrap();
    let arr = match result {
        ParseOutput::Array(v) => v,
        _ => panic!("expected array"),
    };
    assert!(!arr.is_empty(), "nmcli device show should produce entries");
}

#[test]
fn test_nmcli_device_table() {
    let input = get_fixture("centos-7.7/nmcli-device.out");
    let parser = find_parser("nmcli").unwrap();
    let result = parser.parse(&input, true).unwrap();
    let arr = match result {
        ParseOutput::Array(v) => v,
        _ => panic!("expected array"),
    };
    assert!(!arr.is_empty(), "nmcli device table should produce entries");
}

#[test]
fn test_nmcli_empty() {
    let parser = find_parser("nmcli").unwrap();
    let result = parser.parse("", true).unwrap();
    let arr = match result {
        ParseOutput::Array(v) => v,
        _ => panic!("expected array"),
    };
    assert!(arr.is_empty());
}

// ─── IWCONFIG ───────────────────────────────────────────────────────────────

#[test]
fn test_iwconfig_registered() {
    assert!(find_parser("iwconfig").is_some());
}

#[test]
fn test_iwconfig_basic() {
    let input = get_fixture("generic/iwconfig.out");
    let parser = find_parser("iwconfig").unwrap();
    let result = parser.parse(&input, true).unwrap();
    let arr = match result {
        ParseOutput::Array(v) => v,
        _ => panic!("expected array"),
    };
    assert!(!arr.is_empty(), "iwconfig should produce interfaces");
    let first = &arr[0];
    assert!(first.contains_key("name"), "missing name");
    assert!(first.contains_key("essid"), "missing essid");
}

#[test]
fn test_iwconfig_many() {
    let input = get_fixture("generic/iwconfig-many.out");
    let parser = find_parser("iwconfig").unwrap();
    let result = parser.parse(&input, true).unwrap();
    let arr = match result {
        ParseOutput::Array(v) => v,
        _ => panic!("expected array"),
    };
    assert!(
        arr.len() > 1,
        "iwconfig-many should produce multiple interfaces"
    );
}

#[test]
fn test_iwconfig_empty() {
    let parser = find_parser("iwconfig").unwrap();
    let result = parser.parse("", true).unwrap();
    let arr = match result {
        ParseOutput::Array(v) => v,
        _ => panic!("expected array"),
    };
    assert!(arr.is_empty());
}

// ─── IW_SCAN ────────────────────────────────────────────────────────────────

#[test]
fn test_iw_scan_registered() {
    assert!(find_parser("iw_scan").is_some());
}

#[test]
fn test_iw_scan_basic() {
    let input = get_fixture("centos-7.7/iw-scan0.out");
    let parser = find_parser("iw_scan").unwrap();
    let result = parser.parse(&input, true).unwrap();
    let arr = match result {
        ParseOutput::Array(v) => v,
        _ => panic!("expected array"),
    };
    assert!(!arr.is_empty(), "iw scan should produce BSS entries");
    let first = &arr[0];
    assert!(first.contains_key("bssid"), "missing bssid");
}

#[test]
fn test_iw_scan_empty() {
    let parser = find_parser("iw_scan").unwrap();
    let result = parser.parse("", true).unwrap();
    let arr = match result {
        ParseOutput::Array(v) => v,
        _ => panic!("expected array"),
    };
    assert!(arr.is_empty());
}

// ─── ETHTOOL ────────────────────────────────────────────────────────────────

#[test]
fn test_ethtool_registered() {
    assert!(find_parser("ethtool").is_some());
}

#[test]
fn test_ethtool_default() {
    let input = get_fixture("generic/ethtool--default1.out");
    let parser = find_parser("ethtool").unwrap();
    let result = parser.parse(&input, true).unwrap();
    let obj = match result {
        ParseOutput::Object(v) => v,
        _ => panic!("expected object"),
    };
    assert!(obj.contains_key("name"), "missing name");
}

#[test]
fn test_ethtool_module_info() {
    let input = get_fixture("generic/ethtool--module-info.out");
    let parser = find_parser("ethtool").unwrap();
    let result = parser.parse(&input, true).unwrap();
    let obj = match result {
        ParseOutput::Object(v) => v,
        _ => panic!("expected object"),
    };
    assert!(!obj.is_empty(), "ethtool module-info should produce output");
}

#[test]
fn test_ethtool_empty() {
    let parser = find_parser("ethtool").unwrap();
    let result = parser.parse("", true).unwrap();
    let arr = match result {
        ParseOutput::Array(v) => v,
        _ => panic!("expected array"),
    };
    assert!(arr.is_empty());
}

// ─── HTTP_HEADERS ────────────────────────────────────────────────────────────

#[test]
fn test_http_headers_registered() {
    assert!(find_parser("http_headers").is_some());
}

#[test]
fn test_http_headers_example_com() {
    let input = get_fixture("generic/http_headers--example-com.out");
    let parser = find_parser("http_headers").unwrap();
    let result = parser.parse(&input, true).unwrap();
    let arr = match result {
        ParseOutput::Array(v) => v,
        _ => panic!("expected array"),
    };
    assert!(!arr.is_empty(), "http_headers should produce output");
    // Fixture contains a request followed by a response; check at least one has status
    let has_status = arr.iter().any(|r| r.contains_key("_response_status"));
    assert!(
        has_status,
        "should have at least one response with status field"
    );
}

#[test]
fn test_http_headers_empty() {
    let parser = find_parser("http_headers").unwrap();
    let result = parser.parse("", true).unwrap();
    let arr = match result {
        ParseOutput::Array(v) => v,
        _ => panic!("expected array"),
    };
    assert!(arr.is_empty());
}

// ─── CURL_HEAD ──────────────────────────────────────────────────────────────

#[test]
fn test_curl_head_registered() {
    assert!(find_parser("curl_head").is_some());
}

#[test]
fn test_curl_head_example_com() {
    let input = get_fixture("generic/curl_head--ILvs-example-com.out");
    let parser = find_parser("curl_head").unwrap();
    let result = parser.parse(&input, true).unwrap();
    let arr = match result {
        ParseOutput::Array(v) => v,
        _ => panic!("expected array"),
    };
    assert!(!arr.is_empty(), "curl_head should produce output");
    // curl -v output has request (>) and response (<) lines; check for response with status
    let has_status = arr.iter().any(|r| r.contains_key("_response_status"));
    assert!(
        has_status,
        "should have at least one response with status field"
    );
}

#[test]
fn test_curl_head_empty() {
    let parser = find_parser("curl_head").unwrap();
    let result = parser.parse("", true).unwrap();
    let arr = match result {
        ParseOutput::Array(v) => v,
        _ => panic!("expected array"),
    };
    assert!(arr.is_empty());
}
