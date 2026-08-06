import Link from "next/link";
import { site } from "@/lib/site";
import summary from "@/data/summary.json";

const COLUMNS = [
  {
    title: "Reference",
    links: [
      { href: "/parsers", label: `All ${summary.documented} parsers` },
      { href: "/compatibility", label: "How the number is measured" },
      { href: "/install", label: "Install" },
    ],
  },
  {
    title: "Packages",
    links: [
      { href: site.cratesIo, label: "crates.io" },
      { href: site.npm, label: "npm, WebAssembly" },
      { href: site.dockerHub, label: "Docker Hub" },
      { href: site.releases, label: "Static binaries" },
    ],
  },
  {
    title: "Upstream",
    links: [
      { href: site.jc, label: "jc, the schema authority" },
      { href: site.cj, label: "cj, the code this forked" },
      { href: site.repo, label: "Source" },
    ],
  },
];

export function SiteFooter() {
  return (
    <footer className="mt-24 border-t bg-[var(--color-surface)]">
      <div className="mx-auto grid max-w-6xl gap-10 px-5 py-12 sm:grid-cols-2 lg:grid-cols-4">
        <div>
          <p className="font-mono text-sm font-semibold">
            jc<span className="text-[var(--color-key)]">-rs</span>
          </p>
          <p className="mt-2 max-w-56 text-sm text-[var(--color-muted)]">
            {summary.matchRate}% of jc {summary.jcVersion}&apos;s fixture corpus, measured on
            every commit.
          </p>
        </div>

        {COLUMNS.map((col) => (
          <div key={col.title}>
            <p className="font-mono text-[11px] tracking-wide text-[var(--color-faint)] uppercase">
              {col.title}
            </p>
            <ul className="mt-3 space-y-2 text-sm">
              {col.links.map((l) => (
                <li key={l.href}>
                  {l.href.startsWith("/") ? (
                    <Link
                      href={l.href}
                      className="text-[var(--color-muted)] transition-colors hover:text-[var(--color-ink)]"
                    >
                      {l.label}
                    </Link>
                  ) : (
                    <a
                      href={l.href}
                      className="text-[var(--color-muted)] transition-colors hover:text-[var(--color-ink)]"
                    >
                      {l.label}
                    </a>
                  )}
                </li>
              ))}
            </ul>
          </div>
        ))}
      </div>

      <div className="border-t">
        <p className="mx-auto max-w-6xl px-5 py-5 text-xs text-[var(--color-faint)]">
          MIT. jc-rs emits the JSON schemas defined by jc (Kelly Brazil).
          jc-rs invents no schemas: where jc-rs and jc disagree, jc-rs has the bug.
        </p>
      </div>
    </footer>
  );
}
