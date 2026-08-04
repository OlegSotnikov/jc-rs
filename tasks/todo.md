# jc-rs — current state and remaining work

Last updated: 2026-08-04

## Where this stands

Independent project, single root commit on 2026-08-04. Everything below M0/M1 is
done; the rest is not.

**Compatibility right now: 804/934 = 86.1%.** The imported codebase measured
80.0% before the two fixes below.
Run `make differential` to regenerate; `tests/differential/REPORT.md` has the
per-fixture breakdown.

## Done

- [x] **M0 — codebase in place.** Five-crate workspace, binary `jc-rs`,
      metadata pointing at this repo and jc-rs.com. `cargo build --release`
      green.
- [x] **M1 — honest harness.** `tests/differential/validate.py`. A pair counts
      only when jc reproduces its own fixture; everything else is reported by
      category. Two things that make the number trustworthy and were not
      obvious:
      - `tests/fixtures/` is now a **verbatim mirror** of the `jc` submodule
        (`make sync-fixtures`, enforced by `make check-fixtures`). 17 fixtures
        in the imported copy carried the implementation's own output rather than
        jc's; the mirror removes that whole class of problem.
      - the run is pinned to **`TZ=PST8PDT`**, which is what jc's `runtests.sh`
        uses. In any other zone jc cannot reproduce its own `*_epoch` fields and
        146 fixtures silently leave the denominator.
- [x] jc submodule pinned to **v1.25.7** (was v1.25.6).
- [x] Independent project: single root commit, no upstream remotes, our own MIT
      licence. `LICENSE` retains the two upstream copyright lines that MIT
      requires for the imported portions; README carries a one-line
      "inspired by" credit and nothing more.
- [x] **Fix: naive timestamps were converted as UTC** (`jc-rs-utils/src/timestamp.rs`).
      Every `*_epoch` across ~20 parsers was out by the UTC offset. This one fix
      moved the corpus 80.0% → 86.1%.
- [x] **Fix: `ping_s` summary dropped `time_ms`** (parsed into state, never
      serialised). 13 → 32 matching fixtures.

## Blocked

- [ ] **Push to GitHub.** `github.com/OlegSotnikov/jc-rs` exists and is empty,
      but the token in `/home/os/sysadmin/secrets/github.env` is dead
      (`401 Bad credentials`, recorded in that directory's README on
      2026-07-29) and `~/.ssh/aula_github_mirror` is rejected. Needs a fresh PAT
      with `repo` + `workflow` scope written into `GITHUB_TOKEN`, and
      `GITHUB_USER` corrected from `x-access-token` to `OlegSotnikov`. Until
      then everything is committed locally on `master` only.

## Next, in order

- [ ] **M2 — streaming redesign.** Two defects, one cause:
      1. streaming parsers emit a JSON array; jc emits NDJSON, one object per line
      2. nothing is emitted until EOF, so `tail -f … | jc-rs --clf-s` produces
         nothing, ever
      The comment at `crates/jc-rs/src/main.rs:486` explains why: the current
      design cannot downcast `dyn Parser`. The fix is a separate
      `StreamingParser` trait (or enum dispatch) plus a line-driven output path
      honouring `-u/--unbuffer`.
      Affects all 17 `*_s` parsers.
- [ ] **M3 — parity.** 39 parsers still failing. Largest first:
      `mdadm` (33), `git_log` + `git_log_s` (20), `rsync_s` (7), `dir` (7),
      `upower` (6), `ufw` (5), `stat_s` (5), `stat` (4), `ls_s` (4), `ping_s` (4).
      Then the 7 parsers that do not exist at all: `tsv`, `tsv-s`, `tsv-ih`,
      `tsv-ih-s`, `csv-ih`, `csv-ih-s`, `typeset`.
      Also `--proc` autodetect (9 errors) — the entry point for 51 hidden
      `proc_*` parsers and all `/proc/...` magic syntax.
- [ ] **Key order.** We serialise alphabetically; jc preserves schema order.
      Values agree but no two outputs ever `diff` clean. Needs an order-preserving
      map (`serde_json` `preserve_order`) and per-parser key sequences.
- [ ] **M4 — hygiene.** The workspace manifest bulk-disables ~80 clippy lints
      plus `dead_code`, `unused_imports`, `unused_variables`, `unused_mut`.
      Remove them in batches, keep CI at `-D warnings`.
- [ ] **`make test` is red on purpose: 648 pass, 15 fail.** These tests were
      green only because they compared against the edited fixture copy.
      Restoring jc's originals made them fail, which is correct — each one is a
      real defect, and together they are the shortest path into M3:

      ```
      disk::mdadm::tests::test_mdadm_examine_raid1_ok
      disk::mdadm::tests::test_mdadm_query_raid1_ok
      disk::tune2fs::tests::test_tune2fs
      log::git_log::tests::test_git_log_default
      log::git_log::tests::test_git_log_medium
      log::git_log::tests::test_git_log_blank_author_fix
      log::git_log::tests::test_git_log_hash_in_message
      log::git_log::tests::test_git_log_is_hash_regex
      log::git_log_s::tests::test_git_log_s_golden
      network::ethtool::tests::test_ethtool_module_info_golden
      network::iw_scan::tests::test_iw_scan_centos_golden
      network::route::tests::test_route_centos_6n_golden
      network::route_print::tests::test_route_print_win10_golden
      network::route_print::tests::test_route_print_win2016_golden
      network::tracepath::tests::test_tracepath_centos_golden
      ```

      Do not make them pass by touching a fixture. Fix the parser. The suite
      goes green as part of M3 and CI stays red until then, which is the honest
      signal.
- [ ] **M5 — distribution.** GitHub Releases (musl x86_64/aarch64, macOS both
      arches, Windows), five crates on crates.io (all names verified free
      2026-08-04), npm for `jc-rs-wasm`, Homebrew tap, `scratch` Docker image,
      shell completions in the archives, `cargo-binstall` metadata.
- [ ] **M6 — jc-rs.com.** Next.js 16 + Tailwind v4 on webapps-kz behind the
      Cloudflare tunnel. Zone `jc-rs.com` is already active in Cloudflare
      (`cfe25a3a86ec23e886d075a99deab437`); DNS, tunnel route and nginx vhost are
      deliberately **not** created yet — a hostname with no container behind it
      returns 502. Content: install, honest comparison table, the live
      compatibility report, a generated page per parser, WASM playground.
- [ ] **M7 — announce.** Notify Kelly Brazil directly *before*
      any public post. The headline is the audit and the reproducible number,
      not "rewritten in Rust".

## Things that will bite you

- **`make test` must run under `TZ=PST8PDT`** or timestamp tests fail. The
  Makefile does this; running `cargo test` by hand does not.
- **The differential oracle needs jc's optional Python deps** (`xmltodict`,
  `ruamel.yaml`, `pygments`) or ~30 fixtures fall into `oracle_reject`.
  `make deps-py`.
- **Do not "fix" a parser by editing a fixture.** `make check-fixtures` will
  catch it, and it is the exact mistake that produced the 100% claim.
- **149 fixtures are still `unmapped`** — the filename does not resolve to a
  parser name. That is honest reporting, not coverage. Worth reducing.
- `cargo install jc-rs` must never be documented as `cargo install jc`: that
  name belongs to an unrelated 2022 crate.
