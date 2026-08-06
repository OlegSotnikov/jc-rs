#!/usr/bin/env python3
"""Generate the website's data files from the jc-rs repository itself.

Nothing here is hand-maintained. Every number on the site traces back to one of
four sources in this repo, so the site cannot drift from the product:

  jc-rs -a                        parser list, descriptions, versions
  crates/**/*.rs ParserInfo       platforms, tags, magic commands
  tests/differential/report.json  per-parser fixture coverage
  tests/fixtures/                 a real input/output pair per parser

Run via `make site-data`. The output is committed so the Docker build needs
neither Python nor the fixture corpus.
"""

import json
import pathlib
import re
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parents[2]
OUT = ROOT / "website" / "src" / "data"
BIN = ROOT / "target" / "release" / "jc-rs"

sys.path.insert(0, str(ROOT / "tests" / "differential"))
from validate import map_parser  # noqa: E402  (reuses the harness's own mapping)

# Keep an example small enough to read on a phone. Fixtures run to megabytes.
MAX_INPUT = 1400
MAX_OUTPUT = 2600

INFO_RE = re.compile(
    r"ParserInfo\s*=\s*ParserInfo\s*\{(.*?)\n\};", re.S
)
FIELD_RE = re.compile(r'^\s*(\w+):\s*(.*?),\s*$', re.M)


def parse_info_blocks() -> dict[str, dict]:
    """Pull compatible/tags/magic_commands out of the static ParserInfo literals."""
    out: dict[str, dict] = {}
    for path in (ROOT / "crates" / "jc-rs-parsers" / "src").rglob("*.rs"):
        for body in INFO_RE.findall(path.read_text(encoding="utf-8", errors="replace")):
            fields = dict(FIELD_RE.findall(body))
            name = fields.get("name", "").strip('"')
            if not name:
                continue
            out[name] = {
                "platforms": re.findall(r"Platform::(\w+)", fields.get("compatible", "")),
                "tags": re.findall(r"Tag::(\w+)", fields.get("tags", "")),
                "magic": re.findall(r'"([^"]+)"', fields.get("magic_commands", "")),
                "source": str(path.relative_to(ROOT)),
            }
    return out


def pick_examples(names: set[str]) -> dict[str, dict]:
    """One representative fixture pair per parser: the smallest that fits."""
    best: dict[str, dict] = {}
    fixtures = ROOT / "tests" / "fixtures"
    for jf in fixtures.rglob("*.json"):
        platform = jf.parent.name
        base = jf.stem
        parser = map_parser(base, platform, names)
        if parser is None:
            continue
        infile = next(
            (c for ext in (".out", ".log", ".csv", ".pem", ".txt", ".conf", ".ini", "")
             if (c := jf.with_suffix(ext)).exists() and c != jf),
            None,
        )
        if infile is None:
            continue
        try:
            raw_in = infile.read_text(encoding="utf-8")
            raw_out = json.loads(jf.read_text(encoding="utf-8"))
        except (UnicodeDecodeError, json.JSONDecodeError):
            continue
        pretty = json.dumps(raw_out, indent=2, ensure_ascii=False)
        if len(raw_in) > MAX_INPUT or len(pretty) > MAX_OUTPUT or not raw_in.strip():
            continue
        cand = {
            "fixture": str(jf.relative_to(fixtures)),
            "platform": platform,
            "input": raw_in.rstrip("\n"),
            "output": pretty,
            "size": len(raw_in) + len(pretty),
        }
        if parser not in best or cand["size"] < best[parser]["size"]:
            best[parser] = cand
    for v in best.values():
        v.pop("size", None)
    return best


def main() -> int:
    if not BIN.exists():
        sys.exit(f"{BIN} not found; run: make build")

    about = json.loads(subprocess.run([BIN, "-a"], capture_output=True, text=True, check=True).stdout)
    report = json.loads((ROOT / "tests" / "differential" / "report.json").read_text())
    infos = parse_info_blocks()

    names = {p["name"] for p in about["parsers"]}
    examples = pick_examples(names)
    per_parser = report["per_parser"]

    parsers = []
    for p in sorted(about["parsers"], key=lambda x: x["name"]):
        name = p["name"]
        # Registration test scaffolding, not a parser anyone would reach for.
        if name == "dummy":
            continue
        extra = infos.get(name, {})
        cov = per_parser.get(name)
        parsers.append({
            "name": name,
            "argument": p["argument"],
            "description": p["description"],
            "version": p["version"],
            "streaming": p["streaming"],
            "deprecated": p["deprecated"],
            # Hidden parsers are reached through the `proc` meta-parser rather
            # than listed by `-l`. They are documented all the same: they are
            # selectable by name and they carry their own fixtures.
            "hidden": p["hidden"],
            "platforms": extra.get("platforms", []),
            "tags": extra.get("tags", []),
            "magic": extra.get("magic", []),
            "source": extra.get("source"),
            "coverage": {"tested": cov["total"], "match": cov["match"]} if cov else None,
            "example": examples.get(name),
        })

    summary = {
        "jcVersion": report["jc_version"],
        "matchRate": report["match_rate"],
        "tested": report["tested"],
        "matched": report["summary"]["match"],
        "parserCount": about["parser_count"],
        "documented": len(parsers),
        "withCoverage": sum(1 for p in parsers if p["coverage"]),
        "withExample": sum(1 for p in parsers if p["example"]),
        "streaming": sum(1 for p in parsers if p["streaming"]),
        "oracleReject": len(report["oracle_rejects"]),
        "unmapped": len(report["unmapped"]),
        "noInput": len(report["no_input"]),
        "failures": len(report["failures"]),
        "version": about["version"],
    }

    OUT.mkdir(parents=True, exist_ok=True)
    (OUT / "parsers.json").write_text(json.dumps(parsers, ensure_ascii=False, indent=1) + "\n")
    (OUT / "summary.json").write_text(json.dumps(summary, ensure_ascii=False, indent=1) + "\n")

    print(f"parsers.json  {len(parsers)} parsers, {summary['withExample']} with a live example")
    print(f"summary.json  {summary['matchRate']}% over {summary['tested']} pairs, jc {summary['jcVersion']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
