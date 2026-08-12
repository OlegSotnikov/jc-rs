export type GuideCategory = "Formats" | "Pipelines" | "Decisions";

export type Guide = {
  href: `/guides/${string}`;
  title: string;
  description: string;
  category: GuideCategory;
  readingMinutes: number;
  parserHrefs: `/parsers/${string}`[];
};

/**
 * The editorial set is deliberately small and task-shaped. Keeping it here
 * gives the guide index, homepage, footer, and sitemap one source of truth.
 */
export const guides: Guide[] = [
  {
    href: "/guides/ndjson-vs-json-vs-jsonl",
    title: "NDJSON vs JSON vs JSONL",
    description:
      "Choose the right shape for streaming records, and see what changes in a real command-line pipeline.",
    category: "Formats",
    readingMinutes: 9,
    parserHrefs: [
      "/parsers/clf-s",
      "/parsers/git-log-s",
      "/parsers/ls-s",
      "/parsers/syslog-s",
    ],
  },
  {
    href: "/guides/json-vs-yaml-vs-toml",
    title: "JSON vs YAML vs TOML",
    description:
      "A practical format comparison for configuration files, machine output, and shell workflows.",
    category: "Formats",
    readingMinutes: 11,
    parserHrefs: ["/parsers/yaml", "/parsers/toml", "/parsers/xml"],
  },
  {
    href: "/guides/logs-to-json",
    title: "Convert Linux logs to JSON",
    description:
      "Parse syslog, web access logs, CEF events, and live streams without pretending every log has one schema.",
    category: "Pipelines",
    readingMinutes: 10,
    parserHrefs: [
      "/parsers/syslog",
      "/parsers/syslog-s",
      "/parsers/clf",
      "/parsers/clf-s",
      "/parsers/cef",
      "/parsers/cef-s",
    ],
  },
  {
    href: "/guides/bash-jc-rs-jq",
    title: "Bash, jc-rs, and jq",
    description:
      "Turn human-readable command output into JSON first, then query it safely with jq.",
    category: "Pipelines",
    readingMinutes: 8,
    parserHrefs: ["/parsers/df", "/parsers/ps"],
  },
  {
    href: "/guides/git-log-to-json",
    title: "Git log to JSON",
    description:
      "Keep Git's record structure intact without maintaining a fragile pretty-format escape scheme.",
    category: "Pipelines",
    readingMinutes: 7,
    parserHrefs: ["/parsers/git-log"],
  },
  {
    href: "/guides/ascii-table-to-json",
    title: "ASCII table to JSON",
    description:
      "Convert aligned terminal tables into typed records, including the limits that decide whether parsing is safe.",
    category: "Pipelines",
    readingMinutes: 8,
    parserHrefs: [
      "/parsers/asciitable",
      "/parsers/asciitable-m",
      "/parsers/csv",
      "/parsers/kv",
      "/parsers/tsv",
    ],
  },
  {
    href: "/guides/native-json-or-jc-rs",
    title: "Native JSON or jc-rs?",
    description:
      "Prefer a command's stable JSON flag when it has one; use a schema-aware parser when it does not.",
    category: "Decisions",
    readingMinutes: 8,
    parserHrefs: [
      "/parsers/ip-route",
      "/parsers/lsblk",
      "/parsers/syslog",
      "/parsers/systemctl",
    ],
  },
  {
    href: "/guides/parsing-command-output-reliably",
    title: "Why command-output parsers break",
    description:
      "Whitespace, locale, wrapping, versions, and stderr are the failure modes hidden by a one-line awk script.",
    category: "Decisions",
    readingMinutes: 9,
    parserHrefs: ["/parsers/df", "/parsers/ps", "/parsers/ss", "/parsers/systemctl"],
  },
  {
    href: "/guides/curl-headers-to-json",
    title: "curl headers to JSON",
    description:
      "Preserve repeated headers and response boundaries when curl output becomes input to a shell script.",
    category: "Pipelines",
    readingMinutes: 7,
    parserHrefs: ["/parsers/curl-head", "/parsers/http-headers"],
  },
];

export function guidesByCategory(): Array<{ category: GuideCategory; items: Guide[] }> {
  const order: GuideCategory[] = ["Formats", "Pipelines", "Decisions"];
  return order.map((category) => ({
    category,
    items: guides.filter((guide) => guide.category === category),
  }));
}
