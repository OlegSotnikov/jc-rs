#!/usr/bin/env python3
"""Cross-validation harness: compare cj (Rust) output against jc (Python) for all fixtures.

Usage:
    python3 tests/cross_validate.py                    # full run
    python3 tests/cross_validate.py --parser ps        # single parser
    python3 tests/cross_validate.py --report report.md # custom output path
    python3 tests/cross_validate.py --fail-fast        # stop on first mismatch
"""

import argparse
import json
import os
import re
import subprocess
import sys
from collections import defaultdict
from pathlib import Path
from typing import Any, Optional

# -- Config --
PROJECT_ROOT = Path(__file__).resolve().parent.parent
CJ_BIN = PROJECT_ROOT / "target" / "release" / "cj"
JC_ROOT = PROJECT_ROOT / "jc"
FIXTURES_DIR = PROJECT_ROOT / "tests" / "fixtures"

# Add jc to Python path
sys.path.insert(0, str(JC_ROOT))

import jc  # noqa: E402


# -- Fixture discovery --

def find_fixture_pairs() -> list[dict]:
    """Find all .out/.json fixture pairs and map to parser names."""
    pairs = []
    for platform_dir in sorted(FIXTURES_DIR.iterdir()):
        if not platform_dir.is_dir():
            continue
        platform = platform_dir.name

        json_files = sorted(platform_dir.glob("*.json"))
        for json_file in json_files:
            base = json_file.stem  # e.g., "ps-axu", "df-h", "arp"
            out_file = json_file.with_suffix(".out")
            if not out_file.exists():
                # Try other input extensions
                for ext in [".log", ".csv", ".pem", ".txt", ".conf", ".ini"]:
                    alt = json_file.with_suffix(ext)
                    if alt.exists():
                        out_file = alt
                        break
                else:
                    continue  # No input file found

            # Map fixture base name to parser name
            parser_name = fixture_to_parser(base, platform)
            if parser_name:
                pairs.append({
                    "parser": parser_name,
                    "platform": platform,
                    "fixture_base": base,
                    "input_file": out_file,
                    "expected_file": json_file,
                })
    return pairs


def fixture_to_parser(base: str, platform: str) -> Optional[str]:
    """Map a fixture base name to a jc parser module name.

    Examples:
        'ps-axu'       -> 'ps'
        'df-h'         -> 'df'
        'arp-a'        -> 'arp'
        'acpi-V'       -> 'acpi'
        'git-log'      -> 'git_log'
        'apt_cache_show--standard' -> 'apt_cache_show'
        'crontab-u'    -> 'crontab_u'
        'stat'         -> 'proc/stat' (for linux-proc platform)
    """
    jc_parsers = set(jc.parser_mod_list())

    # Handle linux-proc fixtures specially
    if platform == "linux-proc":
        # Map to proc_XXX parser names
        proc_name = f"proc_{base}".replace("-", "_")
        if proc_name in jc_parsers:
            return proc_name
        # Try without numbers: cpuinfo2 -> proc_cpuinfo
        proc_name_stripped = re.sub(r'\d+$', '', proc_name)
        if proc_name_stripped in jc_parsers:
            return proc_name_stripped
        return None

    # Direct match (underscores)
    name_under = base.replace("-", "_")
    if name_under in jc_parsers:
        return name_under

    # Try progressively shorter prefixes (strip variant suffixes)
    # e.g., 'ps-axu' -> try 'ps_axu', then 'ps'
    # But be careful: 'git-log' IS the parser name, not 'git' with variant 'log'
    parts = base.split("-")
    for i in range(len(parts), 0, -1):
        candidate = "_".join(parts[:i])
        if candidate in jc_parsers:
            return candidate

    # Handle double-dash variants: 'apt_cache_show--standard' -> 'apt_cache_show'
    if "--" in base:
        prefix = base.split("--")[0].replace("-", "_")
        if prefix in jc_parsers:
            return prefix

    return None


# -- Parser invocation --

def run_jc(parser_name: str, input_data: str, is_streaming: bool = False) -> Any:
    """Run jc Python parser on input data."""
    try:
        if is_streaming:
            lines = input_data.strip().split("\n")
            results = list(jc.parse(parser_name, lines))
            # Remove _meta from streaming results for comparison
            for r in results:
                if isinstance(r, dict):
                    r.pop("_meta", None)
            return results
        else:
            return jc.parse(parser_name, input_data)
    except Exception as e:
        return {"_error": f"jc error: {type(e).__name__}: {e}"}


def run_cj(parser_name: str, input_data: str) -> Any:
    """Run cj Rust binary on input data."""
    # Convert parser module name to CLI flag: 'git_log' -> '--git-log'
    cli_flag = f"--{parser_name.replace('_', '-')}"
    try:
        result = subprocess.run(
            [str(CJ_BIN), cli_flag],
            input=input_data,
            capture_output=True,
            text=True,
            timeout=10,
        )
        if result.returncode != 0:
            return {"_error": f"cj exit {result.returncode}: {result.stderr.strip()}"}
        output = result.stdout.strip()
        if not output:
            return {"_error": "cj produced empty output"}
        return json.loads(output)
    except subprocess.TimeoutExpired:
        return {"_error": "cj timeout (10s)"}
    except json.JSONDecodeError as e:
        return {"_error": f"cj invalid JSON: {e}"}
    except Exception as e:
        return {"_error": f"cj error: {type(e).__name__}: {e}"}


# -- Comparison --

def deep_diff(expected: Any, actual: Any, path: str = "$") -> list[dict]:
    """Deep comparison of two JSON values, returning list of differences."""
    diffs = []

    if isinstance(expected, dict) and isinstance(actual, dict):
        all_keys = set(expected.keys()) | set(actual.keys())
        for key in sorted(all_keys):
            child_path = f"{path}.{key}"
            if key not in expected:
                diffs.append({"path": child_path, "type": "extra_key", "actual": actual[key]})
            elif key not in actual:
                diffs.append({"path": child_path, "type": "missing_key", "expected": expected[key]})
            else:
                diffs.extend(deep_diff(expected[key], actual[key], child_path))
    elif isinstance(expected, list) and isinstance(actual, list):
        if len(expected) != len(actual):
            diffs.append({
                "path": path,
                "type": "array_length",
                "expected": len(expected),
                "actual": len(actual),
            })
        for i in range(min(len(expected), len(actual))):
            diffs.extend(deep_diff(expected[i], actual[i], f"{path}[{i}]"))
    elif expected != actual:
        # Check if it's just a type difference (e.g., int vs float)
        diff_type = "value_mismatch"
        if type(expected) != type(actual):
            diff_type = "type_mismatch"
        diffs.append({
            "path": path,
            "type": diff_type,
            "expected": repr(expected),
            "actual": repr(actual),
        })

    return diffs


def classify_diffs(diffs: list[dict]) -> dict:
    """Classify diffs by severity."""
    classified = {"P0_structural": [], "P1_type": [], "P2_value": []}
    for d in diffs:
        if d["type"] in ("missing_key", "extra_key", "array_length"):
            classified["P0_structural"].append(d)
        elif d["type"] == "type_mismatch":
            classified["P1_type"].append(d)
        else:
            classified["P2_value"].append(d)
    return classified


# -- Main harness --

def run_validation(
    parser_filter: Optional[str] = None,
    fail_fast: bool = False,
    verbose: bool = False,
) -> dict:
    """Run cross-validation and return results."""
    pairs = find_fixture_pairs()
    if parser_filter:
        pairs = [p for p in pairs if p["parser"] == parser_filter]

    results = {
        "total": 0,
        "match": 0,
        "mismatch": 0,
        "jc_error": 0,
        "cj_error": 0,
        "skipped": 0,
        "details": [],
    }

    # Group by parser for summary
    parser_stats = defaultdict(lambda: {"total": 0, "match": 0, "mismatch": 0, "errors": 0})

    print(f"Found {len(pairs)} fixture pairs to validate\n")

    for i, pair in enumerate(pairs):
        parser = pair["parser"]
        platform = pair["platform"]
        base = pair["fixture_base"]
        label = f"[{parser}] {platform}/{base}"
        is_streaming = parser.endswith("_s")

        results["total"] += 1
        parser_stats[parser]["total"] += 1

        # Read input
        input_data = pair["input_file"].read_text(errors="replace")

        # Read expected output (from fixture .json)
        try:
            expected = json.loads(pair["expected_file"].read_text())
        except json.JSONDecodeError:
            results["skipped"] += 1
            if verbose:
                print(f"  SKIP {label} (invalid fixture JSON)")
            continue

        # Run cj
        cj_result = run_cj(parser, input_data)

        if isinstance(cj_result, dict) and "_error" in cj_result:
            results["cj_error"] += 1
            parser_stats[parser]["errors"] += 1
            detail = {"label": label, "status": "cj_error", "error": cj_result["_error"]}
            results["details"].append(detail)
            if verbose:
                print(f"  ERR  {label}: {cj_result['_error']}")
            if fail_fast:
                break
            continue

        # Compare cj output against fixture expected output
        diffs = deep_diff(expected, cj_result)

        if not diffs:
            results["match"] += 1
            parser_stats[parser]["match"] += 1
            if verbose:
                print(f"  OK   {label}")
        else:
            results["mismatch"] += 1
            parser_stats[parser]["mismatch"] += 1
            classified = classify_diffs(diffs)
            detail = {
                "label": label,
                "status": "mismatch",
                "diff_count": len(diffs),
                "classified": {k: len(v) for k, v in classified.items()},
                "sample_diffs": diffs[:5],
            }
            results["details"].append(detail)
            if verbose:
                p0 = len(classified["P0_structural"])
                p1 = len(classified["P1_type"])
                p2 = len(classified["P2_value"])
                print(f"  DIFF {label}: {len(diffs)} diffs (P0={p0} P1={p1} P2={p2})")
                for d in diffs[:3]:
                    print(f"       {d['path']}: {d['type']}")
            if fail_fast:
                break

        # Progress
        if not verbose and (i + 1) % 50 == 0:
            print(f"  ... {i + 1}/{len(pairs)} checked")

    results["parser_stats"] = dict(parser_stats)
    return results


def generate_report(results: dict, output_path: Path):
    """Generate markdown report."""
    lines = []
    lines.append("# CJ Cross-Validation Report\n")
    lines.append(f"## Summary\n")
    lines.append(f"| Metric | Count |")
    lines.append(f"|--------|-------|")
    lines.append(f"| Total fixture pairs | {results['total']} |")
    lines.append(f"| **Exact match** | **{results['match']}** |")
    lines.append(f"| Mismatch | {results['mismatch']} |")
    lines.append(f"| CJ error | {results['cj_error']} |")
    lines.append(f"| Skipped | {results['skipped']} |")

    match_pct = (results['match'] / results['total'] * 100) if results['total'] else 0
    lines.append(f"\n**Match rate: {match_pct:.1f}%**\n")

    # Parser-level summary
    stats = results.get("parser_stats", {})
    if stats:
        # Sort: fully passing first, then by mismatch count
        sorted_parsers = sorted(
            stats.items(),
            key=lambda x: (-(x[1]["match"] == x[1]["total"]), -x[1]["mismatch"], -x[1]["errors"]),
        )

        # Failing parsers table
        failing = [(p, s) for p, s in sorted_parsers if s["match"] < s["total"]]
        if failing:
            lines.append(f"\n## Failing Parsers ({len(failing)})\n")
            lines.append("| Parser | Total | Match | Mismatch | Error |")
            lines.append("|--------|-------|-------|----------|-------|")
            for parser, s in failing:
                lines.append(f"| `{parser}` | {s['total']} | {s['match']} | {s['mismatch']} | {s['errors']} |")

        # Passing parsers count
        passing = [(p, s) for p, s in sorted_parsers if s["match"] == s["total"]]
        lines.append(f"\n## Passing Parsers ({len(passing)})\n")
        if passing:
            lines.append(", ".join(f"`{p}`" for p, _ in passing))

    # Detailed diffs for failing
    mismatches = [d for d in results["details"] if d["status"] == "mismatch"]
    if mismatches:
        lines.append(f"\n## Mismatch Details (top 30)\n")
        for d in mismatches[:30]:
            lines.append(f"### {d['label']}")
            lines.append(f"Diffs: {d['diff_count']} — {d.get('classified', {})}")
            lines.append("```")
            for sd in d.get("sample_diffs", []):
                lines.append(f"  {sd['path']}: {sd['type']}")
                if "expected" in sd:
                    lines.append(f"    expected: {sd['expected']}")
                if "actual" in sd:
                    lines.append(f"    actual:   {sd['actual']}")
            lines.append("```\n")

    # CJ errors
    errors = [d for d in results["details"] if d["status"] == "cj_error"]
    if errors:
        lines.append(f"\n## CJ Errors ({len(errors)})\n")
        lines.append("| Fixture | Error |")
        lines.append("|---------|-------|")
        for d in errors:
            err_short = d["error"][:100]
            lines.append(f"| {d['label']} | `{err_short}` |")

    output_path.write_text("\n".join(lines) + "\n")
    print(f"\nReport written to {output_path}")


def main():
    parser = argparse.ArgumentParser(description="Cross-validate cj vs jc fixture output")
    parser.add_argument("--parser", help="Filter to a single parser name")
    parser.add_argument("--report", default="tests/cross_validation_report.md", help="Report output path")
    parser.add_argument("--fail-fast", action="store_true", help="Stop on first mismatch")
    parser.add_argument("-v", "--verbose", action="store_true", help="Show per-fixture results")
    args = parser.parse_args()

    # Ensure cj binary exists
    if not CJ_BIN.exists():
        print(f"ERROR: cj binary not found at {CJ_BIN}")
        print("Run: cargo build --release")
        sys.exit(1)

    results = run_validation(
        parser_filter=args.parser,
        fail_fast=args.fail_fast,
        verbose=args.verbose,
    )

    # Print summary
    print(f"\n{'='*60}")
    print(f"RESULTS: {results['match']}/{results['total']} exact match "
          f"({results['match']/results['total']*100:.1f}%)" if results['total'] else "No tests")
    print(f"  Mismatch: {results['mismatch']}  |  CJ Error: {results['cj_error']}  |  Skipped: {results['skipped']}")
    print(f"{'='*60}")

    # Generate report
    report_path = PROJECT_ROOT / args.report
    generate_report(results, report_path)

    # Exit code: 0 if all match, 1 if any failures
    sys.exit(0 if results["mismatch"] == 0 and results["cj_error"] == 0 else 1)


if __name__ == "__main__":
    main()
