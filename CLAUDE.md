# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this project is

`jc-rs` converts the output of ~236 command-line tools, file formats and strings
to JSON, as a single static binary. It is a Rust implementation of the schemas
defined by [jc](https://github.com/kellyjonbrazil/jc) (Python), pinned as a git
submodule at `./jc` (v1.25.7).

**The product is a compatibility number anyone can check, not speed.**
That premise decides most design arguments:

- Never invent a schema. jc is the authority; where the two disagree, jc-rs has the bug.
- Never edit a fixture to make a test pass. `make check-fixtures` will catch it.
- Never exclude an awkward fixture from the count; report it in a category.
- Publish the number even when it is bad.

Current state: 934/934 = 100% (`tests/differential/REPORT.md`). The workspace
version lives in the root `Cargo.toml`; `./ci/release.sh X.Y.Z` bumps it and
every internal pin in one step.

## Commands

```bash
make build              # cargo build --release
make check              # lint + fixture sync + tests + differential; the universal gate
make lint               # cargo clippy --workspace --all-targets -- -D warnings; cargo fmt --check
make test               # unit + integration tests, pinned to TZ=PST8PDT
make differential       # full jc corpus; rewrites tests/differential/{REPORT.md,report.json}
make bench              # criterion, -p jc-rs-bench
make wasm               # build + Node-test the npm package (needs wasm-pack)
make submodule deps-py  # one-time setup: pin the jc oracle + its optional Python deps
```

Narrower runs:

```bash
TZ=PST8PDT cargo test -p jc-rs-parsers disk::mdadm          # one module's tests
TZ=PST8PDT cargo test -p jc-rs --test integration           # CLI integration tests
python3 tests/differential/validate.py --parser mdadm -v    # differential for one parser
python3 tests/differential/validate.py --fail-under 100     # the CI floor
```

`TZ=PST8PDT` is mandatory and non-obvious: jc's fixtures carry `*_epoch` fields
computed in local time and jc's own `runtests.sh` pins that zone. A bare
`cargo test` produces a pile of meaningless timestamp failures, and a
differential run in another zone silently drops 146 pairs from the denominator.
The Makefile, the CI test job and the differential harness all set it;
hand-run cargo does not.

## The compatibility floor

`--fail-under 100` in `.github/workflows/ci.yml` is what stops the number
sliding. Raise the floor in the same commit that raises the number; never lower
it silently.

## Architecture

Five crates, dependency order `core → utils → parsers → jc-rs`:

| Crate | Role |
|---|---|
| `jc-rs-core` | `Parser`/`StreamingParser`/`LineParser` traits, `ParseOutput`, `ParserInfo`, `ParseError`/`CjError`, and the registry |
| `jc-rs-utils` | shared helpers: `simple_table_parse`/`sparse_table_parse`, `convert_to_*`, `normalize_key`, `parse_timestamp`, `slice_lines` |
| `jc-rs-parsers` | every parser, grouped by domain (`disk/ format/ log/ misc/ network/ package/ proc/ security/ string/ system/`) |
| `jc-rs` | the CLI binary: `args`, `magic`, `meta`, `output`, `streaming` |
| `jc-rs-wasm` | `wasm-bindgen` wrapper: `parse`, `parseRaw`, `parsers`, `StreamSession` |
| `jc-rs-bench` | criterion benchmarks |

**Registration is link-time via `inventory`.** There is no central parser list:
each parser declares a `static INFO: ParserInfo`, a `static X_PARSER`, and an
`inventory::submit! { ParserEntry::new(&X_PARSER) }`. The CLI has
`extern crate jc_rs_parsers;` solely to force linking so those submissions run.
Lookup goes through `find_parser()` (accepts `name`, `kebab-case`, or
`--argument`) and `find_magic_parser()` (matches argv against `magic_commands`).

**Streaming state lives in a session object, not on the parser.** The registry holds
`&'static dyn Parser`, which cannot be downcast, so `Parser::as_streaming()`
hands back the sub-trait and `StreamingParser::session()` mints an owned
`LineParser` carrying the per-run state. A streaming parser's `Parser::parse()`
must be `parse_via_session(self, input, quiet)`. The batch path and the live
path have to be the same code, because the differential only exercises the
batch one.

`docs/api-contracts.md` is the authoritative spec for these interfaces: types,
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
codebase's "100% (687/687)" claim came from exactly those two mechanisms: a
harness that dropped 39% of the corpus and 17 rewritten fixtures. Do not
reintroduce either.

Bumping the `jc` submodule is a deliberate act: re-run the differential, expect
the number to move, and update the CI floor.

## Known structural gaps

- **`-r/--raw` is implemented where the corpus proves it.** `Parser::parse_raw`
  defaults to forwarding to `parse`, which is correct wherever jc's `_process`
  is a no-op; seven parsers override it. A parser with conversions but no raw
  fixture is still unproven; if you add one, check `-r` too.
- **Key order.** Keys serialise alphabetically; jc preserves schema order. Values
  agree so the differential passes, but no two outputs ever `diff` clean. Fixing
  it needs `serde_json`'s `preserve_order` plus per-parser key sequences.
- **Lint debt.** Three clippy allows are left in the workspace manifest, each
  with the site count and the reason it has not been cleared: `manual_strip`
  (47), `type_complexity` (7), `if_same_then_else` (2). CI is at `-D warnings`,
  so removing one is self-verifying. Nothing else is bulk-allowed; if you add
  an allow, write down why next to it.
- **Streaming is only live under `-u`.** Without it both jc and jc-rs
  block-buffer stdout when piped; that is jc's behaviour, not an oversight.
  `crates/jc-rs/tests/streaming.rs` covers what the differential structurally
  cannot: its fixtures are arrays and its input ends immediately, so a parser
  that buffers to EOF scores identically to one that streams.
- **`ifconfig` pays 10 ms before it reads anything.** Every pattern a parser
  needs is now compiled once per process rather than once per call, which is
  worth 65× to 129× per parse for `ifconfig` and `iwconfig` — but a CLI parses
  once, so single-shot runs saw none of it. `ifconfig` holds 31 patterns and
  applies all 23 of the detail ones to every line, so nothing is skippable and
  a 3-line input costs exactly what a full dump does. Cutting it means
  detecting the platform from the interface line first and only then touching
  that family's patterns. Parsers whose patterns were rebuilt *per record*
  (`traceroute`, `ufw`, `ping`) did gain end-to-end: 2× to 4×.
  `jc_rs_utils::cached_regex` exists for the case a pattern arrives as an
  argument and cannot be a `static`.

`tests/differential/report.json` has every failing case with paths and diffs.

## Dependencies

Two rules, both enforced rather than remembered:

- **Nothing abandoned ships.** `cargo deny check advisories` runs on every push
  with an empty ignore list, and an upstream archive counts against a crate even
  when no advisory exists. That is why YAML goes through `serde-saphyr` and not
  `serde_yaml`: dtolnay archived `serde_yaml` in March 2024 and it pulls
  `unsafe-libyaml` behind it. The swap costs ~590 KB of binary, which
  `cargo bloat --crates` puts on `granit_parser` (249 KB of `.text`),
  `serde_saphyr` itself (176 KB) and `annotate_snippets` (121 KB); `encoding_rs`
  comes along in the tree but the linker drops nearly all of it. That is the
  price of not shipping an abandoned parser.
  It carries one known deviation from jc that no fixture covers: a zero-padded
  scalar such as `007` types as a float, where jc gives the integer `7`.
  (`serde_yaml` gave the string `"007"`, so this was already wrong before.)
- **A dependency nobody imports is a bug.** v0.5.0 dropped six declared-but-unused
  crates, one of which (`ini`, last released 2020, pinning `configparser` 1.0.0)
  was the only thing standing between this workspace and a Debian archive that
  already packages every other crate it needs. Before adding a crate, grep for
  the one it would replace; before a release, check that each declared dep is
  reachable from code.

## Commits

Every commit is authored and committed by `Oleg Sotnikov <os@g1sw.com>`. Never
add a `Co-Authored-By` trailer or any other attribution line: this is a
single-owner repository and the contributor list has to say so.

## The website

`website/` is jc-rs.com: Next.js 16, statically rendered, deployed as a
container behind the nginx container on the web host. `website/README.md` has
the deploy commands and the infrastructure table.

The thing to know from here: **nothing on the site is hand-written data**.
`website/src/data/*.json` is generated by `website/scripts/build-data.py` from
`jc-rs -a`, the `ParserInfo` literals, `tests/differential/report.json` and
`tests/fixtures/`. Run `make site` after a release, or the site keeps
advertising the previous version and the previous number.

The front page runs `jc-rs-wasm` in the browser, so the converter there is the
same parser set the binary ships. `make site-wasm` rebuilds it;
`website/public/wasm/` is a gitignored build artefact.

## Release and publishing

Cutting a release is a `v*` tag: it fires `release.yml` (five targets, musl
Linux, bash/zsh/fish completions, a deliberately opt-in `jc` alias, checksums,
`scratch` Docker image, Homebrew tap, npm) and `publish-crates.yml` (crates.io in
dependency order) in parallel. Pushing the tag publishes everything; nothing
waits for a click. Each publishing job names an environment, which buys two
things: the environment accepts deployments only from `v*` tags (a
`workflow_dispatch` from `master` is rejected before any step runs), and its
secrets are visible to that one job instead of every workflow in the repo.

Deliberately *not* configured: an approval gate. This is a single-owner
repository, so a required reviewer would only ask the person who just pushed the
tag to confirm that they pushed the tag. Tagging is the deliberate act; adding a
second one buys nothing and trains you to click through. Restore it with
`gh api -X PUT repos/OWNER/REPO/environments/NAME -f 'reviewers[][type]=User'`
if the repo ever gains a second person with push access.

Every job that needs a credential checks for it first and reports-and-skips
rather than failing, so a release completes with whatever is configured.
Re-running a release is safe everywhere: crates and npm versions already
published are skipped, and an unchanged formula exits before `git commit` can
fail on "nothing to commit".

**Package the workspace, never a crate at a time.** `cargo package -p X`
resolves X's siblings against *crates.io*, so it cannot verify a version that
is not on the index yet, which is every version between the bump and the
upload. `cargo package --workspace --exclude jc-rs-bench` packages the members
into a temporary local registry and verifies each against that, so it holds at
any version. (`jc-rs-bench` is excluded as the one unpublished member: its path
dependencies carry no version, and packaging refuses that.)

- crates.io **and npm** publish over **Trusted Publishing (OIDC)**. There is no
  long-lived registry credential for either. If the workflow ever falls back to
  `CARGO_REGISTRY_TOKEN`, fix the trust config rather than adding the secret back.
- **Never give the npm job a token.** `actions/setup-node` writes
  `_authToken=${NODE_AUTH_TOKEN}` into `.npmrc` whenever `registry-url` is set,
  and npm treats even an *empty* token as an attempt to authenticate instead of
  falling through to OIDC. So `registry-url` is omitted (npmjs is the default
  registry anyway) and neither `NPM_TOKEN` nor `NODE_AUTH_TOKEN` appears in the
  job. Every failure on this path surfaces as `ENEEDAUTH` or a 404 regardless of
  cause, so read that as "check the trusted publisher", not "add a token".
  It needs npm >= 11.5.1, hence node 24; node 22 still ships npm 10.9.x.
  Registered on 2026-08-06 at npmjs.com/package/jc-rs-wasm/access against
  organization `OlegSotnikov`, repository `jc-rs`, workflow `release.yml`,
  environment `npm`. All four have to match or the token exchange returns
  non-200 and npm falls back to token auth, which is what produced `ENEEDAUTH`
  on the two releases before it. jc-rs-wasm@0.2.0 was the first version this
  path published, with a `provenance/v1` attestation tying the tarball to the
  run that built it.
- The Docker Hub overview does not update. `peter-evans/dockerhub-description`
  exits 0 and changes nothing, because `DOCKERHUB_TOKEN` is a push/pull PAT:
  `PATCH /v2/repositories/appmasterio/jc-rs/` answers `insufficient scope` from
  both `/v2/auth/token` and `/v2/users/login`, so it is the token, not the
  endpoint. Replace that secret with a Read/Write/Delete PAT and it works; no
  workflow change is needed.
- Only one long-lived credential remains. `HOMEBREW_TAP_TOKEN` exists because
  `GITHUB_TOKEN` cannot reach another repository, and it is the only long-lived
  secret in the release path.
- Secrets live in GitHub *environments*, never at repository level.
- Third-party actions are pinned to commit SHAs. Keep it that way; some of these
  jobs can publish under our name.
- `CARGO_TERM_COLOR: always` is set in these workflows; never match cargo output
  with an anchored pattern (crate names arrive wrapped in ANSI escapes). The
  re-run guard queries the crates.io API instead.
- The published binary is `jc-rs`, never `jc`, because installing `jc` would
  shadow the original in `PATH`. The crate name `cj` on crates.io belongs to an unrelated
  2022 package; never document or publish under it.
- `LICENSE` keeps two upstream copyright lines. They are the MIT condition for
  the imported parser code and jc's fixture corpus, so they stay.
