import parsersData from "@/data/parsers.json";
import summaryData from "@/data/summary.json";

export type Example = {
  fixture: string;
  platform: string;
  input: string;
  output: string;
};

export type Parser = {
  name: string;
  argument: string;
  description: string;
  version: string;
  streaming: boolean;
  deprecated: boolean;
  hidden: boolean;
  platforms: string[];
  tags: string[];
  magic: string[];
  source: string | null;
  coverage: { tested: number; match: number } | null;
  example: Example | null;
};

export const parsers = parsersData as Parser[];
export const summary = summaryData;

const byName = new Map(parsers.map((p) => [p.name, p]));

export function getParser(name: string): Parser | undefined {
  return byName.get(name);
}

/** URL slug: parser names are snake_case, the web reads better in kebab. */
export function slugOf(p: Parser): string {
  return p.name.replaceAll("_", "-");
}

export function fromSlug(slug: string): Parser | undefined {
  return byName.get(slug.replaceAll("-", "_"));
}

/** The parsers worth putting in front of someone first: real command, real example. */
export function featured(count: number): Parser[] {
  const preferred = ["df", "ps", "ls", "dig", "ss", "netstat", "free", "lsblk", "route", "who"];
  const picked = preferred.map(getParser).filter((p): p is Parser => Boolean(p?.example));
  return picked.slice(0, count);
}

export type Group = { title: string; blurb: string; items: Parser[] };

const DOMAIN_TITLES: Record<string, { title: string; blurb: string }> = {
  system: { title: "System", blurb: "Hosts, processes, users, hardware" },
  network: { title: "Network", blurb: "Interfaces, routes, sockets, DNS" },
  disk: { title: "Disk", blurb: "Block devices, filesystems, arrays" },
  format: { title: "File formats", blurb: "Structured files that are not commands" },
  string: { title: "Strings", blurb: "One value in, its parts out" },
  package: { title: "Packages", blurb: "Distribution package managers and indexes" },
  log: { title: "Logs", blurb: "Line-oriented log formats" },
  security: { title: "Security", blurb: "Firewalls, certificates, permissions" },
  proc: { title: "/proc", blurb: "Linux kernel interface files" },
  misc: { title: "Other", blurb: "Everything that fits nowhere else" },
};

/** Group by the source directory, which is how the crate itself is organised. */
export function grouped(): Group[] {
  const buckets = new Map<string, Parser[]>();
  for (const p of parsers) {
    const dir = p.source?.split("/").at(-2) ?? "misc";
    const key = dir in DOMAIN_TITLES ? dir : "misc";
    const bucket = buckets.get(key);
    if (bucket) bucket.push(p);
    else buckets.set(key, [p]);
  }
  return Object.entries(DOMAIN_TITLES)
    .filter(([key]) => buckets.has(key))
    .map(([key, meta]) => ({
      ...meta,
      items: buckets.get(key)!.sort((a, b) => a.name.localeCompare(b.name)),
    }));
}

export function platformLabel(p: string): string {
  return p === "Aix" ? "AIX" : p === "Darwin" ? "macOS" : p;
}
