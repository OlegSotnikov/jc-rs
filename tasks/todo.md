# jc-rs — current state and remaining work

Last updated: 2026-08-04

## Where this stands

**Compatibility: 934/934 = 100%.** The imported codebase measured 80.0%. Run
`make differential` to regenerate; `tests/differential/REPORT.md` has the
breakdown. CI fails below 100% (`--fail-under 100`), and
`ci/known-failures.txt` is empty — every unit test passes.

What 100% does *and does not* mean: it is 100% of the pairs jc itself can
reproduce. 149 fixtures remain `unmapped` (the filename resolves to no parser),
18 have no input file, and 9 are ones jc cannot reproduce. Those are reported
every run rather than hidden, and reducing the unmapped set will *lower* the
headline number before it raises it. That is the trade the harness was built
to make visible.

## Done

- [x] **M0 — codebase in place.** Five-crate workspace, binary `jc-rs`,
      metadata pointing at this repo and jc-rs.com. `cargo build --release`
      green.
- [x] **M1 — honest harness.** `tests/differential/validate.py`. A pair counts
      only when jc reproduces its own fixture; everything else is reported by
      category. Two things that make the number trustworthy and were not
      obvious:
      - `tests/fixtures/` is a **verbatim mirror** of the `jc` submodule
        (`make sync-fixtures`, enforced by `make check-fixtures`). 17 fixtures
        in the imported copy carried the implementation's own output rather than
        jc's; the mirror removes that whole class of problem.
      - the run is pinned to **`TZ=PST8PDT`**, which is what jc's `runtests.sh`
        uses. In any other zone jc cannot reproduce its own `*_epoch` fields and
        146 fixtures silently leave the denominator.
- [x] jc submodule pinned to **v1.25.7**.
- [x] Independent project: single root commit, no upstream remotes, our own MIT
      licence. `LICENSE` retains the two upstream copyright lines that MIT
      requires for the imported portions.
- [x] **CI/CD on GitHub Actions.** `ci.yml` (fmt, clippy, build on three OSes,
      fixture-sync check, test ratchet, full differential with the report as a
      job summary, crate packaging), `release.yml` (five targets, musl for
      Linux, completions, `jc` alias, checksums, scratch Docker image),
      `publish-crates.yml` (crates.io in dependency order, Trusted Publishing).

- [x] **M2 — streaming, for real.** Streaming parsers emitted one JSON array at
      EOF; jc emits NDJSON as input arrives. The blocker was structural: the
      registry holds `&'static dyn Parser` and a trait object cannot be
      downcast, so the CLI could not reach a line-at-a-time API.
      - `LineParser` is a stateful session (`parse_line(&mut self)` +
        `finalize()`), created per run by `StreamingParser::session()`.
        `Parser::as_streaming()` reaches it without `Any`.
      - `parse_via_session()` drives a session over a whole string, so a
        streaming parser's batch path and its live path are the same code.
      - The CLI reads stdin through `BufReader::lines()`, writes through a
        `BufWriter`, and flushes per record under `-u` — matching jc's
        `flush=self.unbuffer` exactly, verified against jc itself.
      - `crates/jc-rs/tests/streaming.rs` asserts records arrive before EOF.
        The differential cannot: its fixtures are arrays and its input ends
        immediately, so a parser that buffers scores the same as one that
        streams.
- [x] **The seven missing parsers.** `tsv`, `tsv_s`, `tsv_ih`, `tsv_ih_s`,
      `csv_ih`, `csv_ih_s`, `typeset`. The six delimited ones share one
      session, as they share one `DictReader` call in jc.
- [x] **`--proc` autodetect.** Was 11 of jc's 50 signatures; now all of them,
      in jc's order (which matters: smaps/maps and zoneinfo/vmstat each match
      the other's files). 9/9, and the entry point for 51 hidden `proc_*`
      parsers and all `/proc/...` magic syntax works.
- [x] **Timestamps.** Two bugs, both "a naive local time read as UTC":
      `mdadm` and `tune2fs` had their own conversions, and the shared helper
      discarded a parsed `%z` offset the wrong way round. Between them, 55
      fixtures.
- [x] **Parser fixes:** `ls_s` (dropped `size`/`links` unless bare integers),
      `ufw` (never emitted `log`), `ping_s` (no ICMP error table at all;
      `ping -I` banner offsets), `rsync_s` (unanchored patterns, missing
      `epoch`, `--stats` block, decimal size units), `ethtool` (local key
      normaliser produced keys with commas in them), `tracepath` (`\S+`
      matched "no" in "no reply"), `lsattr` (filenames with spaces), `dig`
      (TXT quotes), `timedatectl` (`epoch_utc` read as local).
- [x] **Known failures: 15 → 0.** Each line deleted in the commit that fixed
      the parser. No fixture was edited.
- [x] **100% of the corpus.** The last 21 mismatches were three more
      naive-local-read-as-UTC timestamps (certbot, openvpn, plist), a
      week-of-year calculation that fed the wrong month to a weekday routine,
      and one defect each in stat_s, traceroute_s, route, iptables, m3u,
      rsync, iw_scan, route_print, cbt and iwconfig. plist needed a real
      OpenStep parser: the `plist` crate coerces bare tokens to numbers, and
      that format has none -- `0700` is a string.
- [x] **M4 — the bulk lint allow-list is gone.** 585 warnings underneath it,
      including four regexes compiled in loops, fourteen unreachable
      functions and a duplicate `who` parser that was never registered.

## Next, in order

- [ ] **M5 — distribution.** Homebrew tap, `cargo-binstall` metadata, npm for
      `jc-rs-wasm` (that crate is not written yet). Everything else is wired:
      `release.yml` builds five targets and pushes the scratch image,
      `publish-crates.yml` publishes via Trusted Publishing, and shell
      completions for bash, zsh and fish are in the archives.
- [ ] **`-r/--raw` beyond what the corpus proves.** Seven parsers override
      `parse_raw`; the default forwards to `parse`, which is correct wherever
      jc's `_process` is a no-op. A parser with conversions but no `-raw`
      fixture is unproven either way.
- [ ] **Key order.** We serialise alphabetically; jc preserves schema order.
      Values agree but no two outputs ever `diff` clean.
      **`serde_json`'s `preserve_order` feature is not the answer — measured.**
      It swaps `BTreeMap` for `IndexMap`, and on this corpus that costs 30–40%
      of throughput (csv 10k rows 29 → 38 ms, pkg-index-deb 88 → 120 ms,
      clf 10k lines 193 → 278 ms) because the hasher is SipHash and cannot be
      swapped through serde_json. Doing this properly means an ordered
      serializer with a per-parser key sequence, not a feature flag.
- [ ] **149 fixtures are still `unmapped`** — the filename does not resolve to
      a parser name. That is honest reporting, not coverage. Worth reducing.
- [ ] **M4 — hygiene: three allow entries left.** The bulk list is gone;
      `clippy --workspace --all-targets -- -D warnings` is clean. What remains
      is `manual_strip` (47 sites), `type_complexity` (7) and
      `if_same_then_else` (2), each documented in `Cargo.toml` with its count.
      Removing them is mechanical but wants reading, not a script.
- [ ] **M5 — distribution.** Homebrew tap, `cargo-binstall` metadata, npm for
      `jc-rs-wasm` (that crate is not written yet), and a fish completion — the
      generator exists in `crates/jc-rs/src/completions.rs` but no CLI flag
      reaches it.
- [ ] **M6 — jc-rs.com.** Next.js 16 + Tailwind v4 on the-web-host behind the
      Cloudflare tunnel. Zone is already active; DNS, tunnel route and nginx
      vhost are deliberately **not** created yet — a hostname with no container
      behind it returns 502. Content: install, honest comparison table, the live
      compatibility report, a generated page per parser, WASM playground.
- [ ] **M7 — announce.** Notify Kelly Brazil directly *before*
      any public post. The headline is the reproducible number and the harness,
      not "rewritten in Rust".

## Performance notes

`make bench-vs-jc` times both implementations with one harness; the README
table comes from it. What the profile says today:

- **Startup dominates the win** (~20x). It is mostly Python interpreter start,
  which is why the gap narrows to 3–5x on throughput.
- **`[profile.release]` was missing entirely** until 2026-08-04. Adding
  `lto = "fat"`, `codegen-units = 1`, `panic = "abort"` and `strip` cut
  ~25% off throughput and 40% off the binary (8.7 → 5.2 MB).
- **Format hints matter more than they look.** `parse_timestamp` walks jc's
  34-format table until one parses; without a hint a format near the end costs
  ~30 failed `strptime` attempts *per record*. Every call site now passes the
  same hint set as the jc parser it mirrors — and because hinted formats are
  tried first, this is a correctness contract, not only a speed one.
- **The remaining per-record cost is fields, not parsing.** A 22-field `clf`
  record costs ~12 µs against ~3 µs for a 4-field csv row; that is one `String`
  allocation and one map insert per field. Cutting it means changing how
  records are built, not micro-tuning the parsers.

## Things that will bite you

- **`make test` must run under `TZ=PST8PDT`** or timestamp tests fail. The
  Makefile does this; running `cargo test` by hand does not.
- **The differential oracle needs jc's optional Python deps** (`xmltodict`,
  `ruamel.yaml`, `pygments`) or ~30 fixtures fall into `oracle_reject`.
  `make deps-py`.
- **Do not "fix" a parser by editing a fixture.** `make check-fixtures` will
  catch it, and it is the exact mistake that produced the imported code's 100%
  claim.
- **Streaming is only live with `-u`.** Without it both jc and jc-rs
  block-buffer stdout when piped. That is jc's behaviour, not an oversight.
- **A streaming parser's `parse()` and its live path must stay the same code**
  (`parse_via_session`). If they diverge, the differential — which only
  exercises the batch path — stops saying anything about what a pipe produces.
- `cargo install jc-rs` must never be documented as `cargo install jc`: that
  name belongs to an unrelated 2022 crate.
