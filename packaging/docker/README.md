# jc-rs

Convert the output of command-line tools, file formats and strings to JSON — as
a single static binary in a `scratch` image. No shell, no libc, no package
manager: the image is the binary and its licence, nothing else.

```console
$ ps aux | docker run --rm -i appmasterio/jc-rs --ps | jq '.[0]'
$ docker run --rm -i appmasterio/jc-rs --df < df.txt
$ tail -f access.log | docker run --rm -i appmasterio/jc-rs -u --clf-s
```

## What it is

A Rust implementation of the schemas defined by
[jc](https://github.com/kellyjonbrazil/jc), the tool that decided which commands
are worth parsing and what the JSON for each should look like. jc's one weakness
is the Python runtime: ~160 ms of interpreter startup on every invocation, and a
dependency you cannot put in a `scratch` container. This is that tool as a
static binary.

**Compatibility with jc is 100%** — 934 of 934 oracle-valid fixture pairs from
jc's own corpus, measured on every commit and published whatever it says. CI
fails below 100%, so it cannot drift quietly. The rule that makes the number
mean something: a fixture only counts when jc itself reproduces it exactly, and
everything that does not qualify is reported by category rather than dropped.

## Speed

Median of 5–11 runs against jc 1.25.7 on Python 3.12, same harness both sides
([`ci/bench-vs-jc.sh`](https://github.com/OlegSotnikov/jc-rs/blob/master/ci/bench-vs-jc.sh)):

| Scenario | jc | jc-rs |
|---|---|---|
| Cold start | 138 ms | 7 ms |
| `ps aux`, 110 lines | 156 ms | 12 ms |
| `csv`, 10,000 rows | 200 ms | 41 ms |
| `pkg-index-deb`, 1.5 MB | 309 ms | 88 ms |

The gap is largest at startup, which is why it matters most in loops, git hooks
and per-host automation rather than at an interactive prompt.

## Usage

Same interface as jc — 238 parsers, `-p` to pretty-print, `-y` for YAML, `-r`
for raw output, `-M` for metadata.

```console
$ docker run --rm appmasterio/jc-rs --help
$ docker run --rm appmasterio/jc-rs -l          # list parsers
```

Streaming parsers emit NDJSON as input arrives — one JSON object per line, not
one array at the end. Add `-u` to flush per record, which is what a live pipe
needs:

```console
$ ping example.com | docker run --rm -i appmasterio/jc-rs -u --ping-s
```

Magic syntax (`jc-rs df -h`) needs the command inside the container, and this
image deliberately has nothing else in it. Pipe instead, or use the standalone
binary.

## Tags

- `latest` — the most recent release
- `vX.Y.Z` — a specific release

2.3 MB compressed, `linux/amd64` only today.

## Elsewhere

```sh
cargo binstall jc-rs                    # prebuilt binary, no compile
cargo install jc-rs                     # from source
brew install OlegSotnikov/tap/jc-rs     # macOS and Linux
npm install jc-rs-wasm                  # WebAssembly, browser or Node
```

Static binaries for five targets, with shell completions and `SHA256SUMS`, are
attached to every
[release](https://github.com/OlegSotnikov/jc-rs/releases).

Source, the full compatibility report and what is left to do:
<https://github.com/OlegSotnikov/jc-rs>

MIT. jc-rs implements the JSON schemas defined by
[jc](https://github.com/kellyjonbrazil/jc), the original Python tool.
