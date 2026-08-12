#!/usr/bin/env python3
"""Collect US search demand, trend, difficulty, and intent from DataForSEO."""

from __future__ import annotations

import argparse
import base64
import json
import os
from pathlib import Path
from urllib.request import Request, urlopen


API = "https://api.dataforseo.com/v3"
LOCATION_CODE = 2840  # United States
LANGUAGE_CODE = "en"


def api_call(path: str, payload: list[dict], auth: str) -> tuple[list[dict], float]:
    credentials = base64.b64encode(auth.encode()).decode()
    request = Request(
        f"{API}/{path}",
        data=json.dumps(payload).encode(),
        headers={
            "Authorization": f"Basic {credentials}",
            "Content-Type": "application/json",
        },
        method="POST",
    )
    with urlopen(request, timeout=180) as response:
        data = json.load(response)

    if data.get("status_code") != 20000:
        raise RuntimeError(f"DataForSEO error: {data.get('status_message')}")
    task = data["tasks"][0]
    if task.get("status_code") != 20000:
        raise RuntimeError(f"DataForSEO task error: {task.get('status_message')}")
    return task.get("result") or [], data.get("cost") or 0.0


def growth(months: list[dict] | None) -> list[float] | None:
    if not months or len(months) < 6:
        return None
    ordered = sorted(months, key=lambda month: (month["year"], month["month"]))
    values = [month.get("search_volume") or 0 for month in ordered]
    early = sum(values[:3]) / 3
    recent = sum(values[-3:]) / 3
    ratio = round(recent / early, 1) if early else (99.0 if recent else None)
    return [early, recent, ratio] if ratio is not None else None


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("candidates", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()

    auth = os.environ.get("DATAFORSEO_AUTH")
    if not auth or ":" not in auth:
        raise SystemExit("Set DATAFORSEO_AUTH to login:password")

    keywords = list(
        dict.fromkeys(
            line.strip().lower()
            for line in args.candidates.read_text().splitlines()
            if line.strip()
        )
    )
    rows = {
        keyword: {
            "kw": keyword,
            "vol": 0,
            "cpc": 0.0,
            "g": None,
            "kd": None,
            "intent": None,
        }
        for keyword in keywords
    }
    total_cost = 0.0
    returned = set()

    for start in range(0, len(keywords), 100):
        result, cost = api_call(
            "dataforseo_labs/google/historical_search_volume/live",
            [
                {
                    "keywords": keywords[start : start + 100],
                    "location_code": LOCATION_CODE,
                    "language_code": LANGUAGE_CODE,
                }
            ],
            auth,
        )
        total_cost += cost
        for item in (result[0].get("items") or []):
            keyword = item["keyword"].lower()
            info = item.get("keyword_info") or {}
            row = rows.setdefault(
                keyword,
                {"kw": keyword, "vol": 0, "cpc": 0.0, "g": None, "kd": None, "intent": None},
            )
            row.update(
                {
                    "vol": info.get("search_volume") or 0,
                    "cpc": info.get("cpc") or 0.0,
                    "g": growth(info.get("monthly_searches")),
                }
            )
            returned.add(keyword)

    alive = [row["kw"] for row in rows.values() if row["vol"] > 0]
    if alive:
        result, cost = api_call(
            "dataforseo_labs/google/bulk_keyword_difficulty/live",
            [
                {
                    "keywords": alive,
                    "location_code": LOCATION_CODE,
                    "language_code": LANGUAGE_CODE,
                }
            ],
            auth,
        )
        total_cost += cost
        for item in (result[0].get("items") or []):
            keyword = item["keyword"].lower()
            if keyword in rows:
                rows[keyword]["kd"] = item.get("keyword_difficulty")

        result, cost = api_call(
            "dataforseo_labs/google/search_intent/live",
            [{"keywords": alive, "language_code": LANGUAGE_CODE}],
            auth,
        )
        total_cost += cost
        for item in (result[0].get("items") or []):
            keyword = item["keyword"].lower()
            if keyword in rows:
                rows[keyword]["intent"] = (item.get("keyword_intent") or {}).get("label")

    ordered = sorted(rows.values(), key=lambda row: (-row["vol"], row["kw"]))
    args.output.write_text(json.dumps(ordered, indent=2, ensure_ascii=False) + "\n")
    print(
        f"keywords={len(keywords)} returned={len(returned)} alive={len(alive)} "
        f"cost=${total_cost:.4f} output={args.output}"
    )


if __name__ == "__main__":
    main()
