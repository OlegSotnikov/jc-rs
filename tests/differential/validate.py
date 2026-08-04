#!/usr/bin/env python3
"""Differential validation of jc-rs against the full jc fixture corpus.

This is the project's central quality gate. jc is the schema authority: where
jc-rs and jc disagree, jc-rs has a bug.

The rule that makes the number meaningful:

    A (fixture, parser) pair enters the denominator ONLY when jc itself
    reproduces that fixture exactly.

Everything else is reported by category -- unmapped, no-input, oracle-reject --
and never silently dropped. A harness that quietly skips what it cannot handle
reports 100% while being blind to 39% of the corpus; that is precisely the
mistake this file exists to avoid.

Usage:
    python3 tests/differential/validate.py                 # full run
    python3 tests/differential/validate.py --parser ufw    # one parser
    python3 tests/differential/validate.py --fail-under 100
    python3 tests/differential/validate.py --report docs/COMPATIBILITY.md
"""
from __future__ import annotations

import argparse
import collections
import json
import os
import re
import subprocess
import sys
import time
from pathlib import Path

# jc's fixtures carry `*_epoch` fields computed in local time, and jc's own
# runtests.sh pins that to PST8PDT. Without this, every fixture with a
# timestamp fails the oracle check on any other machine and silently leaves
# the denominator -- which would understate coverage exactly the way cj's
# harness overstated it. Must be set before jc is imported and before the
# binary is spawned (it inherits the environment).
os.environ["TZ"] = "PST8PDT"
time.tzset()

ROOT = Path(__file__).resolve().parents[2]
JC_SRC = ROOT / "jc"
FIXTURES = JC_SRC / "tests" / "fixtures"
BIN = ROOT / "target" / "release" / "jc-rs"

# Input files sit next to the expected .json. jc's corpus is not consistent
# about extensions, and some /proc fixtures have none at all.
INPUT_EXTS = ["", ".out", ".csv", ".tsv", ".ini", ".pem", ".der", ".plist",
              ".xml", ".yaml", ".yml", ".toml", ".srt", ".m3u", ".log", ".txt",
              ".conf", ".sh", ".json"]


def load_jc():
    sys.path.insert(0, str(JC_SRC))
    try:
        import jc  # noqa
    except ImportError:
        sys.exit(
            "cannot import jc.\n"
            "  the jc submodule is not checked out: git submodule update --init\n"
            "  optional deps may also be missing: pip install xmltodict ruamel.yaml pygments"
        )
    return jc


def map_parser(base: str, platform: str, parsers: set[str]) -> str | None:
    """Fixture base name -> jc parser module name.

    'ps-axu' -> 'ps'; 'git-log' -> 'git_log' (the parser, not 'git' + variant);
    linux-proc/'pid_io' -> 'proc_pid_io'.
    """
    if platform == "linux-proc":
        n = ("proc_" + base).replace("-", "_")
        if n in parsers:
            return n
        stripped = re.sub(r"\d+$", "", n)          # cpuinfo2 -> proc_cpuinfo
        if stripped in parsers:
            return stripped
        return "proc" if "proc" in parsers else None

    if base.replace("-", "_") in parsers:
        return base.replace("-", "_")

    parts = base.split("-")
    for i in range(len(parts), 0, -1):
        candidate = "_".join(parts[:i])
        if candidate in parsers:
            return candidate

    if "--" in base:                                # apt_cache_show--standard
        prefix = base.split("--")[0].replace("-", "_")
        if prefix in parsers:
            return prefix
    return None


def discover(parsers: set[str]):
    pairs, unmapped, no_input = [], [], []
    for platform_dir in sorted(p for p in FIXTURES.iterdir() if p.is_dir()):
        for expected in sorted(platform_dir.glob("*.json")):
            base = expected.stem
            streaming = raw = False
            stem = base
            if stem.endswith("-streaming"):
                stem, streaming = stem[: -len("-streaming")], True
            if stem.endswith("-raw"):
                stem, raw = stem[: -len("-raw")], True

            inp = next((c for ext in INPUT_EXTS
                        if (c := expected.parent / (stem + ext)).exists() and c != expected),
                       None)
            if inp is None:
                no_input.append(f"{platform_dir.name}/{expected.name}")
                continue

            parser = map_parser(stem, platform_dir.name, parsers)
            if parser is None:
                unmapped.append(f"{platform_dir.name}/{expected.name}")
                continue
            if streaming:
                if parser + "_s" not in parsers:
                    unmapped.append(f"{platform_dir.name}/{expected.name} (no _s parser for {parser})")
                    continue
                parser += "_s"

            pairs.append(dict(parser=parser, platform=platform_dir.name, base=base,
                              input=inp, expected=expected, raw=raw, streaming=streaming))
    return pairs, unmapped, no_input


def run_jc(jc, parser: str, data: bytes, raw: bool, streaming: bool):
    """The oracle. Returns (ok, value_or_message)."""
    text = data.decode("utf-8", errors="replace")
    try:
        if streaming:
            out = list(jc.parse(parser, text.splitlines(), raw=raw, quiet=True))
            for record in out:
                if isinstance(record, dict):
                    record.pop("_meta", None)
            return True, out
        return True, jc.parse(parser, text, raw=raw, quiet=True)
    except Exception as exc:                                    # noqa: BLE001
        return False, f"{type(exc).__name__}: {exc}"


def run_bin(parser: str, data: bytes, raw: bool):
    cmd = [str(BIN), "--" + parser.replace("_", "-"), "-q"]
    if raw:
        cmd.append("-r")
    try:
        proc = subprocess.run(cmd, input=data, capture_output=True, timeout=30)
    except subprocess.TimeoutExpired:
        return False, "timeout after 30s"
    if proc.returncode != 0:
        return False, f"exit {proc.returncode}: {proc.stderr.decode(errors='replace').strip()[:200]}"
    out = proc.stdout.decode(errors="replace").strip()
    if not out:
        return False, "empty output"
    try:
        return True, json.loads(out)
    except json.JSONDecodeError:
        pass
    try:                                            # streaming parsers emit NDJSON
        return True, [json.loads(line) for line in out.splitlines() if line.strip()]
    except json.JSONDecodeError as exc:
        return False, f"invalid JSON: {exc}"


def diff(expected, actual, path="$", acc=None, cap=2000):
    if acc is None:
        acc = []
    if len(acc) >= cap:
        return acc
    if isinstance(expected, dict) and isinstance(actual, dict):
        for key in sorted(set(expected) | set(actual)):
            child = f"{path}.{key}"
            if key not in expected:
                acc.append({"path": child, "kind": "extra_key"})
            elif key not in actual:
                acc.append({"path": child, "kind": "missing_key"})
            else:
                diff(expected[key], actual[key], child, acc, cap)
    elif isinstance(expected, list) and isinstance(actual, list):
        if len(expected) != len(actual):
            acc.append({"path": path, "kind": "array_length",
                        "expected": len(expected), "actual": len(actual)})
        for i in range(min(len(expected), len(actual))):
            diff(expected[i], actual[i], f"{path}[{i}]", acc, cap)
    elif expected != actual:
        # int/float equality is not a schema difference
        if isinstance(expected, (int, float)) and isinstance(actual, (int, float)) \
                and not isinstance(expected, bool) and not isinstance(actual, bool) \
                and float(expected) == float(actual):
            return acc
        acc.append({"path": path,
                    "kind": "type_mismatch" if type(expected) is not type(actual) else "value_mismatch",
                    "expected": repr(expected)[:120], "actual": repr(actual)[:120]})
    return acc


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--parser", help="restrict to one parser")
    ap.add_argument("--report", default="tests/differential/REPORT.md")
    ap.add_argument("--json", dest="json_out", default="tests/differential/report.json")
    ap.add_argument("--fail-under", type=float, default=None,
                    help="exit 1 if the match rate is below this percentage")
    ap.add_argument("-v", "--verbose", action="store_true")
    args = ap.parse_args()

    if not BIN.exists():
        sys.exit(f"{BIN} not found — run: cargo build --release")

    jc = load_jc()
    parsers = set(jc.parser_mod_list(show_hidden=True, show_deprecated=True))
    pairs, unmapped, no_input = discover(parsers)
    if args.parser:
        pairs = [p for p in pairs if p["parser"] == args.parser]

    counts = collections.Counter()
    per_parser = collections.defaultdict(collections.Counter)
    failures, oracle_rejects = [], []

    print(f"jc {jc.__version__} · {len(pairs)} pairs · {len(parsers)} parsers known")

    for i, pair in enumerate(pairs, 1):
        data = pair["input"].read_bytes()
        try:
            expected = json.loads(pair["expected"].read_text(errors="replace"))
        except json.JSONDecodeError:
            counts["bad_fixture"] += 1
            continue

        ok, value = run_jc(jc, pair["parser"], data, pair["raw"], pair["streaming"])
        if not ok or diff(expected, value):
            oracle_rejects.append({"platform": pair["platform"], "base": pair["base"],
                                   "parser": pair["parser"],
                                   "reason": value if not ok else "jc output != its own fixture"})
            counts["oracle_reject"] += 1
            continue
        counts["oracle_ok"] += 1
        per_parser[pair["parser"]]["total"] += 1

        ok, value = run_bin(pair["parser"], data, pair["raw"])
        if not ok:
            counts["error"] += 1
            per_parser[pair["parser"]]["error"] += 1
            failures.append({"platform": pair["platform"], "base": pair["base"],
                             "parser": pair["parser"], "status": "error", "detail": value})
            continue

        differences = diff(expected, value)
        if not differences:
            counts["match"] += 1
            per_parser[pair["parser"]]["match"] += 1
        else:
            counts["mismatch"] += 1
            per_parser[pair["parser"]]["mismatch"] += 1
            kinds = collections.Counter(d["kind"] for d in differences)
            failures.append({"platform": pair["platform"], "base": pair["base"],
                             "parser": pair["parser"], "status": "mismatch",
                             "diff_count": len(differences),
                             "kinds": dict(kinds), "sample": differences[:5]})
        if args.verbose or i % 100 == 0:
            print(f"  {i}/{len(pairs)}", file=sys.stderr)

    tested = counts["match"] + counts["mismatch"] + counts["error"]
    rate = 100.0 * counts["match"] / tested if tested else 0.0

    report = {
        "jc_version": jc.__version__,
        "summary": dict(counts),
        "tested": tested,
        "match_rate": round(rate, 2),
        "per_parser": {k: dict(v) for k, v in sorted(per_parser.items())},
        "failures": failures,
        "oracle_rejects": oracle_rejects,
        "unmapped": unmapped,
        "no_input": no_input,
    }
    Path(ROOT / args.json_out).write_text(json.dumps(report, indent=1))
    write_markdown(ROOT / args.report, report)

    print(f"\nmatch {counts['match']} · mismatch {counts['mismatch']} · error {counts['error']}")
    print(f"match rate over oracle-valid pairs: {rate:.1f}%  ({tested} pairs)")
    print(f"reported but not tested: oracle_reject={counts['oracle_reject']} "
          f"unmapped={len(unmapped)} no_input={len(no_input)}")
    broken = {k: v for k, v in per_parser.items() if v["mismatch"] or v["error"]}
    if broken:
        print(f"\nparsers with failures ({len(broken)}):")
        for name in sorted(broken):
            v = broken[name]
            print(f"  {name:24s} total={v['total']:3d} match={v['match']:3d} "
                  f"mismatch={v['mismatch']:3d} error={v['error']:3d}")

    if args.fail_under is not None and rate < args.fail_under:
        print(f"\nFAIL: {rate:.1f}% < required {args.fail_under}%")
        return 1
    return 0


def write_markdown(path: Path, report: dict) -> None:
    c = report["summary"]
    lines = [
        "# jc-rs compatibility report",
        "",
        f"Generated by `tests/differential/validate.py` against jc {report['jc_version']}.",
        "",
        "A fixture pair counts only when jc reproduces its own fixture exactly.",
        "Everything else is listed below rather than dropped.",
        "",
        "| Metric | Count |",
        "|---|---|",
        f"| Pairs tested | {report['tested']} |",
        f"| Match | {c.get('match', 0)} |",
        f"| Mismatch | {c.get('mismatch', 0)} |",
        f"| Error | {c.get('error', 0)} |",
        f"| **Match rate** | **{report['match_rate']}%** |",
        f"| Excluded: jc could not reproduce its own fixture | {c.get('oracle_reject', 0)} |",
        f"| Excluded: fixture name maps to no parser | {len(report['unmapped'])} |",
        f"| Excluded: no input file found | {len(report['no_input'])} |",
        "",
    ]
    broken = {k: v for k, v in report["per_parser"].items() if v.get("mismatch") or v.get("error")}
    if broken:
        lines += ["## Parsers with failures", "",
                  "| Parser | Total | Match | Mismatch | Error |", "|---|---|---|---|---|"]
        lines += [f"| `{k}` | {v['total']} | {v.get('match', 0)} | {v.get('mismatch', 0)} | {v.get('error', 0)} |"
                  for k, v in sorted(broken.items())]
        lines.append("")
        lines += ["## Failing fixtures", "", "| Parser | Fixture | Status | Detail |", "|---|---|---|---|"]
        for f in report["failures"]:
            detail = f.get("detail") or f"{f.get('diff_count')} diffs: " + \
                ", ".join(f"{v}× {k}" for k, v in (f.get("kinds") or {}).items())
            lines.append(f"| `{f['parser']}` | {f['platform']}/{f['base']} | {f['status']} | {detail[:160]} |")
        lines.append("")
    else:
        lines += ["All oracle-valid fixture pairs match.", ""]
    path.write_text("\n".join(lines))


if __name__ == "__main__":
    sys.exit(main())
