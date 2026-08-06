import Link from "next/link";
import { site } from "@/lib/site";
import summary from "@/data/summary.json";

const COLUMNS = [
  {
    title: "Reference",
    links: [
      { href: "/parsers", label: `All ${summary.documented} parsers` },
      { href: "/compatibility", label: "How the number is measured" },
      { href: "/compare", label: "Compared with jc" },
      { href: "/install", label: "Install" },
    ],
  },
  {
    title: "Get it",
    links: [
      { href: site.cratesIo, label: "crates.io" },
      { href: site.npm, label: "npm, WebAssembly" },
      { href: site.dockerHub, label: "Docker Hub" },
      { href: site.releases, label: "Static binaries" },
    ],
  },
  {
    title: "Project",
    links: [
      { href: site.repo, label: "Source" },
      { href: `${site.repo}/issues`, label: "Issues" },
      { href: `${site.repo}/releases`, label: "Changelog" },
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
            {summary.documented} parsers in one static binary. Verified against the full
            reference corpus on every commit.
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
        <div className="mx-auto flex max-w-6xl flex-wrap items-center justify-between gap-x-6 gap-y-2 px-5 py-5 text-xs text-[var(--color-faint)]">
          <p>
            MIT. jc-rs emits the JSON schemas defined by{" "}
            <a href={site.jc} className="underline-offset-4 hover:underline">
              jc
            </a>
            , the original Python tool, so anything already reading that JSON keeps working.
          </p>
          <p className="shrink-0">
            Created by{" "}
            <a
              href={site.authorUrl}
              className="text-[var(--color-muted)] underline-offset-4 transition-colors hover:text-[var(--color-ink)] hover:underline"
            >
              {site.author}
            </a>
          </p>
        </div>
      </div>
    </footer>
  );
}
