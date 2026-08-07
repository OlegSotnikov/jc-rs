#!/usr/bin/env python3
"""Draw the jc-vs-jc-rs chart the README embeds, from the measured data.

Two files, because GitHub serves one markdown to readers on both themes and an
SVG that assumes either one is unreadable on the other. `<picture>` with
`prefers-color-scheme` picks between them.

The colours are the site's own, snapped into the lightness band the dataviz
validator enforces for each surface: the dark steps are chosen against
`--color-surface` in dark mode rather than being an automatic lightening of the
light ones. Run `node scripts/validate_palette.js` from the dataviz skill if you
change them.

Horizontal bars because the category labels are words, grouped rather than
stacked because the two numbers are alternatives and never sum to anything.
Every bar is labelled with its value: at a linear scale a 5 ms bar next to a
495 ms one is a sliver, and the sliver is the point, so the number has to be
readable independently of the geometry.
"""

import json
import pathlib

ROOT = pathlib.Path(__file__).resolve().parent.parent
DATA = ROOT / "website/src/data/benchmarks.json"

THEMES = {
    "light": {
        "path": ROOT / "docs/bench-light.svg",
        "surface": "#ffffff",
        "ink": "#0f1319",
        "muted": "#5a6474",
        "rule": "#dfe4ea",
        "rs": "#2d5bd0",
        "jc": "#bd5400",
    },
    "dark": {
        "path": ROOT / "docs/bench-dark.svg",
        "surface": "#131922",
        "ink": "#e7ecf3",
        "muted": "#97a3b6",
        "rule": "#212936",
        "rs": "#4f7fdb",
        "jc": "#cf7f3c",
    },
}

W = 760
PAD_X = 16
LABEL_W = 168
# Wide enough that the value label on the longest bar clears the speedup
# column: the peak bar spans the whole plot, so its label lands in this gutter.
VALUE_W = 100
ROW_H = 52
BAR_H = 14
BAR_GAP = 6  # >= the 2px surface gap the spec asks for between adjacent fills
TOP = 84
BOTTOM = 34


def esc(s: str) -> str:
    return (
        s.replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;")
    )


def render(data: dict, t: dict) -> str:
    rows = data["rows"]
    height = TOP + ROW_H * len(rows) + BOTTOM
    plot_x = PAD_X + LABEL_W
    plot_w = W - plot_x - PAD_X - VALUE_W
    peak = max(max(r["jc"], r["rs"]) for r in rows) or 1

    o = [
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{W}" height="{height}" '
        f'viewBox="0 0 {W} {height}" role="img" '
        f'aria-label="Milliseconds per run, jc against jc-rs, lower is better">',
        f'<rect width="{W}" height="{height}" fill="{t["surface"]}"/>',
        f'<style>text{{font-family:ui-sans-serif,-apple-system,"Segoe UI",Roboto,'
        f'Helvetica,Arial,sans-serif}}.n{{font-family:ui-monospace,SFMono-Regular,'
        f'Menlo,Consolas,monospace;font-variant-numeric:tabular-nums}}</style>',
        f'<text x="{PAD_X}" y="30" fill="{t["ink"]}" font-size="16" '
        f'font-weight="600">Milliseconds per run, lower is better</text>',
        f'<text x="{PAD_X}" y="50" fill="{t["muted"]}" font-size="12">'
        f'{esc(data["method"])} · jc {esc(data["jcVersion"])} on Python '
        f'{esc(data["python"])} · {esc(data["platform"])}</text>',
    ]

    # Legend. Two series always get one, so identity is never colour alone.
    lx = PAD_X
    for key, name in (("jc", "jc"), ("rs", "jc-rs")):
        o.append(f'<rect x="{lx}" y="62" width="10" height="10" rx="2" fill="{t[key]}"/>')
        o.append(
            f'<text x="{lx + 16}" y="71" fill="{t["muted"]}" font-size="12">{name}</text>'
        )
        lx += 16 + len(name) * 7 + 18

    for i, r in enumerate(rows):
        top = TOP + i * ROW_H
        mid = top + ROW_H / 2

        if i:
            o.append(
                f'<line x1="{PAD_X}" y1="{top - 4}" x2="{W - PAD_X}" y2="{top - 4}" '
                f'stroke="{t["rule"]}" stroke-width="1"/>'
            )

        o.append(
            f'<text x="{PAD_X}" y="{mid - 3}" fill="{t["ink"]}" font-size="13" '
            f'font-weight="500">{esc(r["scenario"])}</text>'
        )
        o.append(
            f'<text x="{PAD_X}" y="{mid + 13}" fill="{t["muted"]}" font-size="11">'
            f'{esc(r["detail"])}</text>'
        )

        for k, y in (("jc", mid - BAR_H - BAR_GAP / 2), ("rs", mid + BAR_GAP / 2)):
            ms = r[k]
            # A floor so a 5 ms bar against a 495 ms peak is still a visible
            # mark rather than nothing; the number beside it carries the value.
            w = max(round(plot_w * ms / peak), 3)
            o.append(
                f'<rect x="{plot_x}" y="{y:.0f}" width="{w}" height="{BAR_H}" '
                f'rx="4" fill="{t[k]}"/>'
            )
            o.append(
                f'<text x="{plot_x + w + 8}" y="{y + BAR_H - 3:.0f}" '
                f'fill="{t["muted"]}" font-size="11" class="n">{ms} ms</text>'
            )

        speed = r["jc"] / max(r["rs"], 1)
        o.append(
            f'<text x="{W - PAD_X}" y="{mid + 5}" fill="{t["ink"]}" font-size="13" '
            f'font-weight="600" text-anchor="end" class="n">{speed:.1f}×</text>'
        )

    o.append(
        f'<text x="{PAD_X}" y="{height - 12}" fill="{t["muted"]}" font-size="11">'
        f'Measured {esc(data["measured"])} by ci/bench-vs-jc.sh. '
        f'Re-run it with make bench-vs-jc.</text>'
    )
    o.append("</svg>")
    return "\n".join(o) + "\n"


def main() -> None:
    data = json.loads(DATA.read_text())
    for name, t in THEMES.items():
        t["path"].parent.mkdir(parents=True, exist_ok=True)
        t["path"].write_text(render(data, t))
        print(f"{t['path'].relative_to(ROOT)}  {len(data['rows'])} scenarios ({name})")


if __name__ == "__main__":
    main()
