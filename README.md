# jc-rs

Convert the output of command-line tools, file formats and strings to JSON — as a
single static binary.

```console
$ ps aux | jc-rs --ps | jq '.[0]'
$ df -h   | jc-rs --df
$ dig example.com | jc-rs -p --dig
```

> **Status: pre-release.**
> Compatibility with jc: **100%** (934 of 934 oracle-valid fixture pairs).
> The code this started from measured 80.0%. The number comes from
> `make differential` and is published whatever it says — see
> [tests/differential/REPORT.md](tests/differential/REPORT.md).
> CI fails below 100%.

---

## Why this project exists

jc is the standard for this job and the reason the category exists — it decided
which commands are worth parsing and what the JSON for each should look like.
Its one weakness is the Python runtime: ~160 ms of startup on every invocation,
and a dependency you cannot put in a `scratch` container or on an embedded box.

jc-rs is that tool as a single static binary, and it is built around one
commitment:

1. **A compatibility number you can check.** Published continuously, honest when
   it is bad, never rounded up by excluding awkward fixtures. jc-rs invents no
   schemas — jc is the authority, and where the two disagree jc-rs has the bug.
2. **Streaming that actually streams.** jc emits NDJSON line-by-line as input
   arrives, and so does jc-rs: `tail -f access.log | jc-rs -u --clf-s` prints
   each record as the log grows, rather than one array at EOF.
3. **Distribution.** Static binaries for five targets, five crates, npm, brew and
   a `scratch` Docker image.

## How the compatibility number is produced

`tests/differential/validate.py` walks every `.json` fixture in the pinned jc
source (submodule at `./jc`, currently **v1.25.7**) and applies one rule:

> A fixture pair enters the denominator **only when jc itself reproduces that
> fixture exactly.**

Everything that does not qualify is reported by category — `oracle_reject`,
`unmapped`, `no_input` — and never dropped from the count.

Two details decide whether the number means anything:

- **`tests/fixtures/` is a verbatim mirror of the submodule**, enforced by
  `make check-fixtures`. Testing against a fixture copy you are free to edit is
  not testing.
- **The run is pinned to `TZ=PST8PDT`**, which is what jc's own `runtests.sh`
  uses. jc's fixtures carry `*_epoch` fields computed in local time; in any
  other zone the oracle rejects every timestamp-bearing fixture and 146 pairs
  quietly leave the denominator — understating coverage exactly the way a
  silent skip overstates it.

```console
$ make differential
jc 1.25.7 · 943 pairs · 236 parsers known
match 934 · mismatch 0 · error 0
match rate over oracle-valid pairs: 100.0%  (934 pairs)
reported but not tested: oracle_reject=9 unmapped=149 no_input=18
```

## Speed

Measured by [`ci/bench-vs-jc.sh`](ci/bench-vs-jc.sh), which times both sides with
one harness on the same inputs — median of 5–11 runs, Linux x86-64, jc 1.25.7 on
Python 3.12. Re-run it yourself with `make bench-vs-jc`; a number you cannot
reproduce is a number nobody should print.

| Scenario | jc | jc-rs | Speedup |
|---|---|---|---|
| Cold start (`-v`) | 138 ms | 7 ms | **19.7×** |
| `ps aux`, 110 lines | 156 ms | 12 ms | **13.0×** |
| `csv`, 10,000 rows | 200 ms | 41 ms | **4.9×** |
| `pkg-index-deb`, 1.5 MB | 309 ms | 88 ms | **3.5×** |
| `clf`, 10,000 log lines | 604 ms | 208 ms | **2.9×** |

The gap is largest at startup, which is why it matters most in loops, git hooks
and per-host automation rather than at an interactive prompt. On throughput it
narrows to 3–5×: both implementations end up bound by the same per-field work,
and `clf` — 22 fields and a timestamp per line — is the honest floor.

## Usage

Same interface as jc.

```sh
# standard syntax
df -h    | jc-rs --df
ps aux   | jc-rs --ps
mount    | jc-rs --yaml-out --mount

# magic syntax — jc-rs runs the command itself
jc-rs -p df -h
jc-rs -p /proc/meminfo

# line slicing (zero-based, exclusive end)
cat log.txt | jc-rs 2: --syslog

# streaming — one JSON object per record, as it arrives (-u to flush per line)
tail -f access.log | jc-rs -u --clf-s
ping example.com  | jc-rs -u --ping-s

# with jq
ss -tlnp | jc-rs --ss | jq '[.[].local_port] | unique'
```

## Crates

| Crate | What |
|---|---|
| [`jc-rs`](crates/jc-rs) | the CLI binary |
| [`jc-rs-core`](crates/jc-rs-core) | parser traits, output and error types, registry |
| [`jc-rs-parsers`](crates/jc-rs-parsers) | every parser — the reuse surface for other tools |
| [`jc-rs-utils`](crates/jc-rs-utils) | shared helpers: column tables, coercion, key normalisation |
| [`jc-rs-wasm`](crates/jc-rs-wasm) | `wasm-bindgen` wrapper + npm package |

The binary is `jc-rs`, not `jc`. Release archives contain a `jc` alias you can
enable deliberately; nothing installs it by default, because it would shadow the
original jc in `PATH`.

## Use it as a library

```rust
let output = jc_rs_parsers::parse("df", df_output)?;

// Streaming parsers hand back a session you feed a line at a time.
let mut session = jc_rs_parsers::session("clf_s").unwrap();
for line in reader.lines() {
    if let Some(record) = session.parse_line(&line?, true)? {
        handle(record);
    }
}
```

Parsers register themselves at link time, so depending on `jc-rs-parsers` is
what fills the registry — `parse`, `find`, `parsers` and `session` exist so you
never have to think about that.

## Shell completions

```sh
jc-rs -B > /etc/bash_completion.d/jc-rs
jc-rs -Z > "${fpath[1]}/_jc-rs"
jc-rs -F > ~/.config/fish/completions/jc-rs.fish
```

## Install

```sh
cargo binstall jc-rs                    # prebuilt binary, no compile
cargo install jc-rs                     # from source
brew install OlegSotnikov/tap/jc-rs     # macOS and Linux
npm install jc-rs-wasm                  # WebAssembly, browser or Node
docker run --rm -i appmasterio/jc-rs --ps < ps.txt
```

Or take a static binary straight from the
[releases](https://github.com/OlegSotnikov/jc-rs/releases): five targets, with
completions, a `jc` alias and `SHA256SUMS` in every archive.

## Build from source

```console
git clone --recurse-submodules https://github.com/OlegSotnikov/jc-rs.git
cd jc-rs
make build          # cargo build --release
make check          # lint + fixture sync + test ratchet + full differential run
```

The differential suite needs the jc submodule and the Python packages jc's own
parsers use: `make submodule deps-py`.

## License

MIT. See [LICENSE](LICENSE).

---

<sub>inspired by [jc](https://github.com/kellyjonbrazil/jc), mit.</sub>
