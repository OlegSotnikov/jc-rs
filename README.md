# jc-rs

Convert the output of command-line tools, file formats and strings to JSON — as a
single static binary.

```console
$ ps aux | jc-rs --ps | jq '.[0]'
$ df -h   | jc-rs --df
$ dig example.com | jc-rs -p --dig
```

> **Status: pre-release. Do not use this yet.**
> Current compatibility with jc: **86.1%** (804 of 934 fixture pairs).
> Baseline of the code this started from was 80.0%.
> v0.1.0 ships when that number is 100% and streaming is real. The number comes
> from `make differential` and is published whatever it says — see
> [tests/differential/REPORT.md](tests/differential/REPORT.md).

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
   arrives, so `tail -f access.log | jc-rs --clf-s` has to work.
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
match 804 · mismatch 106 · error 24
match rate over oracle-valid pairs: 86.1%  (934 pairs)
reported but not tested: oracle_reject=9 unmapped=149 no_input=18
```

## Speed

Linux x86-64, jc 1.25.7 on Python 3.12, 20 runs each (5 for the large input):

| Scenario | jc | jc-rs | Speedup |
|---|---|---|---|
| Cold start (`-v`) | 163.4 ms | 4.1 ms | **40×** |
| `ps aux`, 110 lines | 166.6 ms | 6.6 ms | **25×** |
| `pkg-index-deb`, 29,735 lines / 1.5 MB | 336.6 ms | 63.8 ms | **5.3×** |

On the large input the two produce byte-identical output (1,755 records). The gap
on small inputs is Python interpreter startup, which is why it matters most in
loops, git hooks and per-host automation rather than at an interactive prompt.

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
| `jc-rs-wasm` | `wasm-bindgen` wrapper + npm package (planned) |

The binary is `jc-rs`, not `jc`. Release archives contain a `jc` alias you can
enable deliberately; nothing installs it by default, because it would shadow the
original jc in `PATH`.

## Build from source

```console
git clone --recurse-submodules https://github.com/OlegSotnikov/jc-rs.git
cd jc-rs
make build          # cargo build --release
make check          # lint + tests + full differential run
```

The differential suite needs the jc submodule and the Python packages jc's own
parsers use: `make submodule deps-py`.

## License

MIT. See [LICENSE](LICENSE).

---

<sub>inspired by [jc](https://github.com/kellyjonbrazil/jc), mit.</sub>
