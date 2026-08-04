# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this project is

`jc-rs` converts the output of ~236 command-line tools, file formats and strings
to JSON, as a single static binary. It is a Rust implementation of the schemas
defined by [jc](https://github.com/kellyjonbrazil/jc) (Python), pinned as a git
submodule at `./jc` (v1.25.7).

**The product is not speed — it is a compatibility number anyone can check.**
That premise decides most design arguments:

- Never invent a schema. jc is the authority; where the two disagree, jc-rs has the bug.
- Never edit a fixture to make a test pass. `make check-fixtures` will catch it.
- Never exclude an awkward fixture from the count; report it in a category.
- Publish the number even when it is bad.

Current state: 804/934 = 86.1% (`tests/differential/REPORT.md`), workspace
version `0.0.0` (the crates.io releases exist only to hold the names).

## Commands

```bash
make build              # cargo build --release
make check              # lint + fixture sync + tests + differential — the universal gate
make lint               # cargo clippy --workspace --all-targets -- -D warnings; cargo fmt --check
./ci/run-tests.sh       # unit tests as a ratchet — THIS is the test gate, not `make test`
make differential       # full jc corpus; rewrites tests/differential/{REPORT.md,report.json}
make bench              # criterion, -p jc-rs-bench
make submodule deps-py  # one-time setup: pin the jc oracle + its optional Python deps
```

Narrower runs:

```bash
TZ=PST8PDT cargo test -p jc-rs-parsers disk::mdadm          # one module's tests
TZ=PST8PDT cargo test -p jc-rs --test integration           # CLI integration tests
python3 tests/differential/validate.py --parser mdadm -v    # differential for one parser
python3 tests/differential/validate.py --fail-under 86.0    # the CI floor
```

`TZ=PST8PDT` is mandatory and non-obvious: jc's fixtures carry `*_epoch` fields
computed in local time and jc's own `runtests.sh` pins that zone. A bare
`cargo test` produces a pile of meaningless timestamp failures, and a
differential run in another zone silently drops 146 pairs from the denominator.
The Makefile and both harnesses set it; hand-run cargo does not.

## The two ratchets

Both fail the build in CI, and both exist because a plain green/red signal would
be useless while the port is incomplete.

**`ci/known-failures.txt`** lists the unit tests that currently fail (real parser
defects, exposed when `tests/fixtures/` became a verbatim mirror of jc's corpus).
`ci/run-tests.sh` fails on a *new* failure **and** on a listed test that starts
passing — when you fix a parser, delete its line in the same commit. The file
should only ever get shorter. `make test` on its own is red by design.

**`--fail-under 86.0`** in `.github/workflows/ci.yml`. Raise the floor in the
same commit that raises the number; never lower it silently.

## Architecture

Five crates, dependency order `core → utils → parsers → jc-rs`:

| Crate | Role |
|---|---|
| `jc-rs-core` | `Parser`/`StreamingParser` traits, `ParseOutput`, `ParserInfo`, `ParseError`/`CjError`, and the registry |
| `jc-rs-utils` | shared helpers: `simple_table_parse`/`sparse_table_parse`, `convert_to_*`, `normalize_key`, `parse_timestamp`, `slice_lines` |
| `jc-rs-parsers` | every parser, grouped by domain (`disk/ format/ log/ misc/ network/ package/ proc/ security/ string/ system/`) |
| `jc-rs` | the CLI binary: `args`, `magic`, `meta`, `output`, `streaming` |
| `jc-rs-bench` | criterion benchmarks |

**Registration is link-time via `inventory`.** There is no central parser list:
each parser declares a `static INFO: ParserInfo`, a `static X_PARSER`, and an
`inventory::submit! { ParserEntry::new(&X_PARSER) }`. The CLI has
`extern crate jc_rs_parsers;` solely to force linking so those submissions run.
Lookup goes through `find_parser()` (accepts `name`, `kebab-case`, or
`--argument`) and `find_magic_parser()` (matches argv against `magic_commands`).

`docs/api-contracts.md` is the authoritative spec for these interfaces — types,
error-variant semantics, naming conventions (`name` is snake_case, `argument` is
`--kebab-case`), the `_jc_meta` object shape, and exit codes (0 ok, 100 error).
Read it before adding to `jc-rs-core` or writing a parser.

Parser unit tests live in each parser file and `include_str!` fixtures straight
out of `tests/fixtures/`, e.g.
`include_str!("../../../../tests/fixtures/generic/swapon-all-v1.out")` compared
against the sibling `.json`.

## How the compatibility number is produced

`tests/differential/validate.py` walks every `.json` fixture in the pinned jc
submodule and applies one rule: **a fixture pair enters the denominator only when
jc itself reproduces that fixture exactly.** Everything else is reported by
category (`oracle_reject`, `unmapped`, `no_input`) and never dropped.

`tests/fixtures/` is a verbatim mirror of the submodule, enforced by
`make check-fixtures` and refreshed with `make sync-fixtures`. Fixtures that are
this project's own test data (no jc counterpart) are left alone; what must never
happen is a fixture jc ships being edited here to match our output. The imported
codebase's "100% (687/687)" claim came from exactly those two mechanisms — a
harness that dropped 39% of the corpus and 17 rewritten fixtures. Do not
reintroduce either.

Bumping the `jc` submodule is a deliberate act: re-run the differential, expect
the number to move, and update the CI floor.

## Known structural gaps

- **Streaming does not stream.** `crates/jc-rs/src/streaming.rs` implements a
  correct line-driven path, but `main.rs` never calls it — the CLI holds a
  `dyn Parser` and cannot downcast to `StreamingParser` (see the "Streaming
  parser path" comment in `main.rs`). All 17 `*_s` parsers therefore emit one
  JSON array at EOF where jc emits NDJSON live. Fixtures are stored as arrays, so
  a green differential does **not** mean streaming works; test it by piping
  slowly and asserting output arrives before EOF.
- **Key order.** Keys serialise alphabetically; jc preserves schema order. Values
  agree so the differential passes, but no two outputs ever `diff` clean. Fixing
  it needs `serde_json`'s `preserve_order` plus per-parser key sequences.
- **Lint debt.** The workspace manifest bulk-allows ~80 clippy lints plus
  `dead_code`, `unused_imports`, `unused_variables`, `unused_mut`. CI is already
  at `-D warnings`, so removing them in batches is self-verifying.

`tasks/todo.md` carries the current milestone list and the per-parser failure
counts; `tests/differential/report.json` has every failing case with paths and
diffs.

## Release and publishing

Cutting a release is a `v*` tag: it fires `release.yml` (five targets, musl
Linux, completions, a deliberately opt-in `jc` alias, checksums, `scratch` Docker
image) and `publish-crates.yml` (crates.io in dependency order) in parallel. Both
stop at a protected environment for human approval, and the environments accept
deployments only from `v*` tags — a `workflow_dispatch` from `master` is rejected
before any step runs.

- crates.io publishing uses **Trusted Publishing (OIDC)**. There is no long-lived
  registry credential anywhere. If the workflow ever falls back to
  `CARGO_REGISTRY_TOKEN`, fix the trust config rather than adding the secret back.
- Secrets live in GitHub *environments*, never at repository level.
- Third-party actions are pinned to commit SHAs. Keep it that way — some of these
  jobs can publish under our name.
- `CARGO_TERM_COLOR: always` is set in these workflows; never match cargo output
  with an anchored pattern (crate names arrive wrapped in ANSI escapes). The
  re-run guard queries the crates.io API instead.
- The published binary is `jc-rs`, never `jc` — installing `jc` would shadow the
  original in `PATH`. The crate name `cj` on crates.io belongs to an unrelated
  2022 package; never document or publish under it.
- `LICENSE` keeps two upstream copyright lines. They are the MIT condition for
  the imported parser code and jc's fixture corpus, not decoration.
