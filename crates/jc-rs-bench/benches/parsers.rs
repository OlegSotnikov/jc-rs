// Criterion benchmarks for jc-rs parsers and utilities.
//
// Covers: df, ps, large CSV streaming, timestamp parsing (cold + warm cache),
// table parsing (simple + sparse), and parser dispatch.

// Force the linker to include all parser registrations (inventory::submit! macros).
extern crate jc_rs_parsers;

use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use jc_rs_core::registry::find_parser;
use jc_rs_utils::{parse_timestamp, simple_table_parse, sparse_table_parse};

// ---------------------------------------------------------------------------
// Static fixtures (embedded at compile time)
// ---------------------------------------------------------------------------

static DF_CENTOS: &str = include_str!("../../../tests/fixtures/centos-7.7/df.out");
static DF_UBUNTU: &str = include_str!("../../../tests/fixtures/ubuntu-18.04/df.out");
static DF_OSX: &str = include_str!("../../../tests/fixtures/osx-10.14.6/df.out");
static PS_AXU: &str = include_str!("../../../tests/fixtures/centos-7.7/ps-axu.out");
static PS_EF: &str = include_str!("../../../tests/fixtures/centos-7.7/ps-ef.out");

// ---------------------------------------------------------------------------
// Helpers that generate synthetic large inputs
// ---------------------------------------------------------------------------

/// Build a large `df`-like table with `rows` data rows.
fn make_large_df_table(rows: usize) -> String {
    let mut s =
        String::from("Filesystem              1K-blocks    Used Available Use% Mounted on\n");
    for i in 0..rows {
        let used = i * 1024;
        let avail = 100_000_000 - used;
        s.push_str(&format!(
            "/dev/sda{:<3}          {}  {}  {}  {}% /mnt/vol{}\n",
            i % 128,
            100_000_000,
            used,
            avail,
            (used * 100 / 100_000_000).min(100),
            i,
        ));
    }
    s
}

/// Build a large ps-like table with `rows` data rows.
fn make_large_ps_table(rows: usize) -> String {
    let mut s = String::from("  PID TTY          TIME CMD\n");
    for i in 0..rows {
        s.push_str(&format!(
            "{:>5} ?        00:{:02}:{:02} process_{}\n",
            i + 1,
            (i / 60) % 60,
            i % 60,
            i,
        ));
    }
    s
}

/// Build a large CSV string with `rows` data rows.
fn make_large_csv(rows: usize) -> String {
    let mut s = String::from("id,name,value,timestamp,description\n");
    for i in 0..rows {
        s.push_str(&format!(
            "{},item_{},{},2024-01-{:02}T{:02}:{:02}:00,description for item {}\n",
            i,
            i,
            i * 42,
            (i % 28) + 1,
            (i / 28) % 24,
            i % 60,
            i,
        ));
    }
    s
}

// ---------------------------------------------------------------------------
// Benchmark: df parser
// ---------------------------------------------------------------------------

fn bench_df(c: &mut Criterion) {
    let parser = find_parser("df").expect("df parser not found");

    let mut group = c.benchmark_group("df");

    group.throughput(Throughput::Bytes(DF_CENTOS.len() as u64));
    group.bench_function("centos_fixture", |b| {
        b.iter(|| {
            let result = parser.parse(black_box(DF_CENTOS), false);
            black_box(result)
        })
    });

    group.throughput(Throughput::Bytes(DF_UBUNTU.len() as u64));
    group.bench_function("ubuntu_fixture", |b| {
        b.iter(|| {
            let result = parser.parse(black_box(DF_UBUNTU), false);
            black_box(result)
        })
    });

    group.throughput(Throughput::Bytes(DF_OSX.len() as u64));
    group.bench_function("osx_fixture", |b| {
        b.iter(|| {
            let result = parser.parse(black_box(DF_OSX), false);
            black_box(result)
        })
    });

    // Scale test: increasing row counts
    for &n in &[50usize, 200, 1000] {
        let input = make_large_df_table(n);
        group.throughput(Throughput::Bytes(input.len() as u64));
        group.bench_with_input(BenchmarkId::new("synthetic_rows", n), &input, |b, inp| {
            b.iter(|| black_box(parser.parse(black_box(inp.as_str()), false)))
        });
    }

    group.finish();
}

// ---------------------------------------------------------------------------
// Benchmark: ps parser
// ---------------------------------------------------------------------------

fn bench_ps(c: &mut Criterion) {
    let parser = find_parser("ps").expect("ps parser not found");

    let mut group = c.benchmark_group("ps");

    group.throughput(Throughput::Bytes(PS_AXU.len() as u64));
    group.bench_function("ps_axu_fixture", |b| {
        b.iter(|| black_box(parser.parse(black_box(PS_AXU), false)))
    });

    group.throughput(Throughput::Bytes(PS_EF.len() as u64));
    group.bench_function("ps_ef_fixture", |b| {
        b.iter(|| black_box(parser.parse(black_box(PS_EF), false)))
    });

    for &n in &[100usize, 500, 2000] {
        let input = make_large_ps_table(n);
        group.throughput(Throughput::Bytes(input.len() as u64));
        group.bench_with_input(BenchmarkId::new("synthetic_rows", n), &input, |b, inp| {
            b.iter(|| black_box(parser.parse(black_box(inp.as_str()), false)))
        });
    }

    group.finish();
}

// ---------------------------------------------------------------------------
// Benchmark: CSV parser (large streaming scenario)
// ---------------------------------------------------------------------------

fn bench_csv(c: &mut Criterion) {
    let parser = find_parser("csv").expect("csv parser not found");

    let mut group = c.benchmark_group("csv");

    for &n in &[100usize, 1000, 10000] {
        let input = make_large_csv(n);
        group.throughput(Throughput::Bytes(input.len() as u64));
        group.bench_with_input(BenchmarkId::new("rows", n), &input, |b, inp| {
            b.iter(|| black_box(parser.parse(black_box(inp.as_str()), false)))
        });
    }

    group.finish();
}

// ---------------------------------------------------------------------------
// Benchmark: timestamp parsing (cold cache vs warm cache)
// ---------------------------------------------------------------------------

fn bench_timestamp(c: &mut Criterion) {
    let mut group = c.benchmark_group("timestamp");

    // Timestamps that exercise different branches of the parser
    let samples = [
        "Wed Mar 24 11:11:30 UTC 2021",
        "2021-03-23 00:14",
        "2003-10-11T22:14:15.003Z",
        "Wed, 31 Jan 2024 00:39:28 GMT",
        "Mon Jan  6 08:49:07 PST 2020",
        "10/06/2019, 09:21:15 AM",
    ];

    // Warm-cache benchmark: parse the same strings repeatedly (they hit LRU)
    group.bench_function("warm_cache_mixed", |b| {
        // Pre-warm the cache
        for s in &samples {
            let _ = parse_timestamp(s, None);
        }
        b.iter(|| {
            for s in &samples {
                black_box(parse_timestamp(black_box(s), None));
            }
        })
    });

    // Per-format benchmarks showing impact of position in the format list
    let early_match = "Wed Mar 24 11:11:30 2021"; // matches fmt id 1000 (~1st)
    let late_match = "2021-03-23 00:14:00 UTC"; // matches fmt id 7255 (near end)

    group.bench_function("early_format_match", |b| {
        b.iter(|| black_box(parse_timestamp(black_box(early_match), None)))
    });

    group.bench_function("late_format_match", |b| {
        b.iter(|| black_box(parse_timestamp(black_box(late_match), None)))
    });

    // ISO 8601 with Z (common case)
    group.bench_function("iso8601_zulu", |b| {
        b.iter(|| black_box(parse_timestamp(black_box("2003-10-11T22:14:15.003Z"), None)))
    });

    // With format hint (skips search, hits the hint directly)
    group.bench_function("with_format_hint", |b| {
        b.iter(|| {
            black_box(parse_timestamp(
                black_box("2021-03-23 00:14"),
                Some("%Y-%m-%d %H:%M"),
            ))
        })
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// Benchmark: table utilities (simple_table_parse + sparse_table_parse)
// ---------------------------------------------------------------------------

fn bench_table_utils(c: &mut Criterion) {
    let mut group = c.benchmark_group("table_utils");

    // Simple table parse with growing row counts
    for &n in &[50usize, 200, 1000] {
        let input = make_large_ps_table(n);
        group.throughput(Throughput::Bytes(input.len() as u64));
        group.bench_with_input(
            BenchmarkId::new("simple_table_parse", n),
            &input,
            |b, inp| b.iter(|| black_box(simple_table_parse(black_box(inp.as_str())))),
        );
    }

    // Sparse table parse with growing row counts (df-style)
    for &n in &[50usize, 200, 1000] {
        let input = make_large_df_table(n);
        group.throughput(Throughput::Bytes(input.len() as u64));
        group.bench_with_input(
            BenchmarkId::new("sparse_table_parse", n),
            &input,
            |b, inp| b.iter(|| black_box(sparse_table_parse(black_box(inp.as_str())))),
        );
    }

    group.finish();
}

// ---------------------------------------------------------------------------
// Benchmark: parser dispatch (find_parser lookup)
// ---------------------------------------------------------------------------

fn bench_parser_dispatch(c: &mut Criterion) {
    let mut group = c.benchmark_group("parser_dispatch");

    let names = ["df", "ps", "csv", "date", "ls", "uname", "free", "mount"];

    group.bench_function("find_by_name", |b| {
        b.iter(|| {
            for name in &names {
                black_box(find_parser(black_box(name)));
            }
        })
    });

    // Worst case: a name that doesn't match any parser
    group.bench_function("find_nonexistent", |b| {
        b.iter(|| black_box(find_parser(black_box("nonexistent_parser_xyz"))))
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// Register all benchmarks
// ---------------------------------------------------------------------------

criterion_group!(
    benches,
    bench_df,
    bench_ps,
    bench_csv,
    bench_timestamp,
    bench_table_utils,
    bench_parser_dispatch,
);
criterion_main!(benches);
