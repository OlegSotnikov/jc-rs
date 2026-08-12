# SEO keyword and content research (deep revision, August 2026)

Research date: 2026-08-11. Market: Google US, English.

## Bottom line

The largest organic opportunities for jc-rs are not generic blog posts. They
are functional, private, in-browser tools already backed by parsers in the
product:

1. a JWT decoder;
2. a CRT/X.509 certificate viewer and decoder;
3. a CSR decoder;
4. XML, CSV, and YAML viewers/converters;
5. a URL parser;
6. TOML, TSV, plist, and other smaller converters.

The strongest editorial opportunity remains a single authoritative guide to
NDJSON, JSONL, and JSON. A second strong guide should compare JSON, YAML, and
TOML for configuration and command-line workflows.

jc-rs already exposes 237 parser pages, so it does not need hundreds of thin
articles. It needs to turn the highest-demand parser pages into tools that
satisfy their SERPs, then publish a small number of guides that join several
closely related wordings on one URL.

Before either, submit and inspect the existing 242-URL sitemap in Google Search
Console. The domain had no detectable Google rankings or indexed results when
checked, and the repository's deployment notes still mark sitemap submission
as manual work outstanding.

## Why this is a revision

The first pass contained 432 hand-built and autocomplete-expanded candidates.
A catalogue coverage audit showed that only about 46 of the 237 parser families
were represented by a direct named query. Semantic coverage was somewhat
higher (`asciitable` appeared as `ASCII table`, for example), but the dataset
was still not exhaustive enough to call complete.

The revised pass generated query families for every parser name, description,
and magic command. It covers conversion, parsing, viewing, decoding, reading,
inspection, online-tool, CLI, JSON-option, structured-output, and
machine-readable-output wording. The resulting measured matrix contains 8,830
unique candidates; 354 have measurable US demand. We then expanded the live
clusters through DataForSEO suggestions, related searches, category ideas,
SERP review, and exact-page competitor gaps.

That extra work materially changed the recommendation. It uncovered JWT,
certificate, CSR, viewer, URL-parser, TOML/TSV, and format-comparison demand
that the first report missed.

No keyword method can prove that no wording exists outside its corpus. This
revision does provide auditable coverage of every feature in the current
parser catalogue and multiple independent discovery routes beyond it.

## Measured opportunities

Volume is average monthly Google US demand. KD is DataForSEO keyword difficulty
from 0 to 100. CPC is included where it is useful as a proxy for commercial
value; it is not a revenue forecast. Closely related terms overlap and must not
be added together as unique traffic.

| Query | US/mo | KD | CPC | Product fit and asset |
|---|---:|---:|---:|---|
| CRT viewer | 40,500 | 29 | $2.15 | `/parsers/x509-cert` tool, after `.crt`/DER input validation |
| JWT decoder / JWT decode | 18,100 | 36 | $30.75 | private local `/parsers/jwt` decoder |
| XML viewer | 3,600 | 30 | $0.61 | secondary intent for the XML converter page |
| certificate decoder | 2,900 | 29 | $22.92 | `/parsers/x509-cert` tool |
| CSR decoder | 2,900 | 10 | $14.02 | `/parsers/x509-csr` tool |
| XML to JSON | 2,900 | 0 | $0.01 | XML converter page |
| CSV to JSON | 2,400 | 12 | $4.62 | CSV converter page |
| CSV viewer | 2,400 | 7 | $2.02 | secondary intent for the CSV page |
| JWT token decode | 2,400 | 35 | $10.30 | same JWT decoder URL |
| NDJSON | 1,600 | 16 | $13.84 | NDJSON/JSONL/JSON pillar guide |
| YAML to JSON | 1,600 | 1 | $0.97 | YAML converter page |
| decode JWT | 1,600 | 37 | — | same JWT decoder URL |
| XML parser | 1,300 | 16 | $7.00 | secondary XML page wording |
| OpenSSL view/read/inspect certificate | 1,000 each | 14–18 | — | CLI section on X.509 tool page |
| YAML vs JSON | 1,000 | 4 | $30.88 | format-comparison guide |
| SSL certificate decoder | 880 | 29 | — | same X.509 tool page |
| JWT parser / JWT parse | 720 | 28–36 | — | same JWT decoder URL |
| URL parser | 720 | 25 | — | private `/parsers/url` tool |
| CSV parser | 590 | 19 | $6.17 | same CSV page |
| JSON vs JSONL / JSONL vs JSON | 590 each | 0–1 | — | NDJSON pillar guide |
| JWT decoder online | 590 | 22–27 | $21.45 | same local JWT tool |
| JSON vs YAML | 480 | 0 | — | same format-comparison guide |
| YAML parser | 480 | 41 | — | same YAML page |
| YAML vs TOML | 320 | 0 | — | same format-comparison guide |
| JSON Lines | 260 | 7 | — | NDJSON pillar guide |
| YAML viewer | 260 | 7 | $5.80 | secondary YAML page wording |
| plist viewer | 210 | 0 | $10.95 | `/parsers/plist` tool |
| NDJSON vs JSON | 210 | 2 | — | NDJSON pillar guide |
| bash jq | 210 | 20 | — | jc-rs + jq workflow guide |
| log to JSON | 140 | 0 | — | Linux log hub |
| TOML to JSON | 90 | 5 | — | `/parsers/toml` tool |
| table to JSON | 90 | 12 | — | ASCII table tool/guide |
| TSV to JSON | 50 | 0 | — | `/parsers/tsv` tool |

The historical growth ratio stored in the JSON compares the earliest three
months returned by the historical endpoint with the latest three. It is not a
simple month-over-month trend. For example, `CRT viewer` grew from an almost
zero early baseline, while its recent twelve months are already relatively
stable around 33,000–40,500 searches.

## SERP and competitor validation

The page type is clear from the live results:

- `JWT decoder` is a tool SERP led by jwt.io, SuperTokens, FusionAuth, jwt.ms,
  and similar interactive decoders. jwt.io's homepage ranks for 323 measured
  terms, including `JWT decode`, `JWT token decode`, `JWT parser`, and online
  variants.
- `certificate decoder`, `CSR decoder`, and `CRT viewer` are tool SERPs.
  SSLShopper's certificate page ranks for 390 measured terms and its CSR page
  for 120. The same certificate page ranks for the 1,000-volume OpenSSL
  inspection variants, so the CLI explanation belongs on the functional page.
- `TOML to JSON` and `TSV to JSON` are almost pure converter SERPs. The leading
  exact competitor pages have very narrow keyword footprints, which makes a
  focused working tool more appropriate than a generic article.
- `JSON vs JSONL` is an editorial SERP with small specialist sites, Reddit,
  IBM documentation, and jsonlines.org. This is the best article-shaped gap.
- `YAML vs JSON` is an editorial comparison SERP led by Reddit, AWS, Stack
  Overflow, and educational sites. One rigorous comparison can target the
  JSON/YAML/TOML/XML variants without creating four duplicate pages.
- `URL parser` mixes interactive tools and language documentation. A working
  browser parser is a valid page type; a prose-only article is not.

## Product-page queue

### P0: build before most articles

1. **Private JWT Decoder — decode locally in your browser**  
   Upgrade `/parsers/jwt` with the real WebAssembly parser, sample token,
   paste input, structured header/payload/signature panes, copy/download, CLI
   example, and a precise privacy statement. The current parser decodes but
   does not cryptographically verify a signature. Say **decode, not verify**
   prominently; do not target `JWT validator` until verification exists.

2. **CRT Viewer and X.509 Certificate Decoder**  
   Upgrade `/parsers/x509-cert`. The parser extracts version, serial, issuer,
   validity, subject, public-key information, extensions, and signature data
   from PEM/DER X.509 certificates. Test real PEM `.crt`, DER `.crt`, file
   upload, browser/WASM, and CLI paths before claiming every format in the
   title. Add an `openssl view certificate` section, but do not call it an SSL
   checker or validator unless those checks are implemented.

3. **CSR Decoder**  
   Upgrade `/parsers/x509-csr` around `CSR decoder`, `certificate request
   decoder`, and `CSR viewer`. The current parser can expose request fields.
   Avoid `CSR checker` and `verify CSR` promises until signature and semantic
   validation are explicitly tested and surfaced.

4. **XML, CSV, and YAML to JSON converters**  
   Upgrade `/parsers/xml`, `/parsers/csv`, and `/parsers/yaml` with paste,
   upload, sample, copy, and download. Use intent-matched titles, for example
   `XML to JSON — private in-browser converter and CLI`. Viewer/parser wording
   can appear naturally in the body; the primary promise remains conversion.

5. **URL Parser**  
   Upgrade `/parsers/url` to show scheme, credentials, host, port, normalized
   path, filename, extension, query values, and fragment. The code decodes
   percent-encoded query keys and values, but the 22,200-volume `URL decoder`
   SERP expects a general-purpose percent encoder/decoder. Do not target that
   head term unless the UI deliberately implements and exposes that task.

### P1: smaller tools

- `/parsers/toml`: TOML to JSON;
- `/parsers/tsv`: TSV to JSON and TSV viewer;
- `/parsers/plist`: plist viewer and plist to JSON;
- `/parsers/srt`: structured SRT viewer/converter, if the UI actually renders
  useful subtitle cues;
- `/parsers/asciitable` and `/parsers/asciitable-m`: table to JSON;
- `/parsers/syslog`, `/parsers/clf`, and `/parsers/cef`: parsing examples linked
  from the log guide.

Every tool page should explain accepted inputs, limits, errors, privacy,
whether validation is performed, and the equivalent CLI invocation. Add
breadcrumbs and structured data only where its required fields truthfully
describe the page.

## Article queue

1. **NDJSON vs JSON vs JSONL: the practical guide for streaming command
   output**  
   Targets: `ndjson`, `json vs jsonl`, `jsonl vs json`, `ndjson vs json`,
   `ndjson vs jsonl`, `json lines`, `difference between json and jsonl`, and
   conversion variants. Cover arrays versus one-object-per-line, latency,
   memory, recovery, backpressure, and real `jc-rs -u` pipelines. Keep all
   comparison word orders on this one URL.

2. **JSON vs YAML vs TOML: choosing a format for config and CLI pipelines**  
   Targets: `yaml vs json`, `json vs yaml`, `yaml vs toml`, and the XML
   comparison variants. Compare comments, types, duplicate keys, streaming,
   human editing, schema/validation, ecosystem support, and conversion. Link
   to the three working converter pages.

3. **How to convert Linux logs to JSON: syslog, CLF, CEF and application
   logs**  
   Targets: `log to json`, `convert log file to json`, and `log parser tool`.
   Distinguish parsing existing logs from configuring software to emit JSON.

4. **Git log to JSON without fragile pretty-format escaping**  
   Exact demand is below the measurable floor, but the SERP is weak and the
   task is a direct `git log | jc-rs --git-log | jq ...` demonstration.

5. **Convert an ASCII table to JSON on the command line**  
   Target `table to json` and the honest structured subset of `text to json`.
   Cover simple and multiline tables; explicitly reject arbitrary prose as an
   input with an inferable schema.

6. **Bash, jc-rs and jq: safely query human-readable command output**  
   Target `bash jq` and `jq bash`. jc-rs creates JSON; jq filters it. Include
   quoting, `pipefail`, error streams, empty input, and tested pipelines.

7. **Native JSON flags or jc-rs? A command-by-command decision guide**  
   Recommend native modes such as `ip -j`, `lsblk -J`, and
   `journalctl -o json` wherever they are stable; explain where a schema-based
   parser remains useful.

8. **Why parsing command output with awk and sed eventually breaks**  
   A linkable engineering article about locale, whitespace, wrapping, version
   drift, error streams, and schema contracts.

9. **curl headers to JSON for shell scripts**  
   Support `/parsers/curl-head`; cover duplicate headers and redirects.

The certificate/OpenSSL and JWT explanations should initially live on their
tool pages. Split a supporting article only after Search Console shows a
distinct informational query set; otherwise the pages will cannibalize each
other.

## False positives removed by SERP review

Programmatic expansion is intentionally broad. These high-volume rows are not
opportunities:

- `free viewer` (60,500), `jobs inspector` (2,400), `time converter online`
  (2,400), and `airport viewer` (1,300) are collisions with short parser or
  command names;
- `URL decoder` (22,200) is primarily percent-encoding/decoding, not merely
  parsing a URL into fields;
- `jc online` (1,600) is Journal & Courier and college navigation, while
  `jc tool` (210) refers to unrelated products;
- `df to JSON` (70) means a pandas DataFrame, not the Linux `df` command;
- `hash decoder` (880) does not describe a truthful reversible operation for
  a cryptographic hash;
- generic `file viewer`, `file parser`, and `file converter` terms do not have
  a specific input contract;
- broad `txt/text to JSON` SERPs often promise arbitrary conversion. jc-rs
  parses known structures and must not make that claim;
- `bash parse JSON` is downstream JSON consumption. It is relevant only inside
  the precise jc-rs-then-jq workflow;
- `syslog viewer` expects a persistent collection/search application, not a
  one-shot parser;
- `CSR checker`, `JWT validator`, and certificate validation terms require
  actual verification behavior, not just decoding.

## Architecture and cannibalization rules

- Keep the existing canonical parser URLs; change their intent and depth, not
  their addresses.
- Add a `/guides` index and a small guide content model with author/reviewer,
  `datePublished`, `dateModified`, Article structured data, and two-way links
  to the parsers demonstrated.
- Put synonyms and word-order variants into one page. Do not create separate
  URLs for `JWT decode`, `decode JWT`, and `JWT decoder`, or for every JSONL
  comparison order.
- Add a visible homepage heading containing **convert command output to JSON**
  while retaining the current distinctive hero.
- Extend `/compare` to resolve jc vs jc-rs vs jq. Do not call jc-rs a jq
  alternative: the tools perform different stages of the pipeline.
- Use real fixtures and executable examples. Avoid 237 articles that restate
  the existing parser reference pages.

## 90-day order

**Day 0:** verify the domain property in Search Console, submit
`https://jc-rs.com/sitemap.xml`, inspect the homepage plus representative
parser URLs, and record submitted/discovered/indexed counts.

**Weeks 1–2:** build the reusable per-parser converter component and upgrade
JWT, X.509 certificate, CSR, XML, CSV, and YAML pages. Test privacy claims,
input formats, errors, metadata, canonicals, and generated HTML.

**Weeks 3–4:** publish the NDJSON/JSONL pillar and the JSON/YAML/TOML comparison.
Add contextual links in both directions between tools and guides.

**Month 2:** ship URL, TOML, TSV, plist, and table tools; publish the log, Git
log, and Bash+jq guides.

**Month 3:** choose expansions from Search Console impressions rather than
from theoretical 10-volume variants. Measure valid indexed pages, non-brand
impressions, top-20/top-10 terms, tool runs, copied commands, and install or
release clicks.

## Method and reproducibility

The deep pass used:

- all 237 parser records: name, description, magic commands, tags, and task
  semantics;
- 8,830 unique generated and discovered candidate phrases;
- DataForSEO historical volume, CPC, KD, and intent for the full matrix;
- 20 focused keyword-suggestion seeds and 12 depth-two related-search seeds;
- a 1,000-row category-idea pass across 21 core product terms;
- six additional exact-cluster expansions for JWT, certificate, CSR, JSONL,
  YAML/JSON, and URL parsing;
- 45 live US SERPs in total: 19 in the first pass and 26 in the revision;
- ranked-keyword gaps for the Python jc sites and three close jc pages, then
  nine exact competitor pages (936 returned rankings) for the newly discovered
  tool and guide clusters.

The scope is US English. A UK/AU/CA or non-English expansion is a separate
market decision, not silently implied by these figures. Search Console remains
the source of truth after indexing.

Files:

- `seo-keyword-candidates-2026-08.txt` — 8,830 normalized candidates;
- `seo-keywords-2026-08.json` — one row per candidate with volume, CPC,
  historical growth, KD, and intent;
- `seo-serp-evidence-2026-08.json` — normalized top organic results for all 45
  reviewed SERPs plus 936 rankings from nine exact competitor pages;
- `seo-keyword-collect.py` — credential-free collector for a repeat run;
- this document — SERP-filtered decisions and execution order.

Re-run from the repository root without storing credentials in the repository:

```bash
read -rsp 'DataForSEO login:password: ' DATAFORSEO_AUTH
export DATAFORSEO_AUTH
echo
python3 docs/seo-keyword-collect.py \
  docs/seo-keyword-candidates-2026-08.txt \
  docs/seo-keywords-2026-08.json
unset DATAFORSEO_AUTH
```

The original research pass cost approximately $0.52. The catalogue-wide
revision, expansions, SERPs, and exact competitor-page checks added about
$2.29, for approximately $2.81 total. A future full matrix refresh should cost
roughly $1.25–$1.35 at the current endpoint pricing before any optional SERP or
competitor refresh.
