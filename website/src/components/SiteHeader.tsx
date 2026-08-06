import Link from "next/link";
import { ThemeToggle } from "@/components/ThemeToggle";
import { site } from "@/lib/site";
import summary from "@/data/summary.json";

const NAV = [
  { href: "/parsers", label: "Parsers" },
  { href: "/compatibility", label: "Compatibility" },
  { href: "/install", label: "Install" },
];

export function SiteHeader() {
  return (
    <header className="sticky top-0 z-40 border-b bg-[color-mix(in_oklab,var(--color-paper)_88%,transparent)] backdrop-blur">
      <div className="mx-auto flex h-14 max-w-6xl items-center gap-6 px-5">
        <Link href="/" className="font-mono text-sm font-semibold tracking-tight">
          jc<span className="text-[var(--color-key)]">-rs</span>
        </Link>

        <nav
          className="hidden gap-5 text-sm text-[var(--color-muted)] sm:flex"
          itemScope
          itemType="https://schema.org/SiteNavigationElement"
        >
          {NAV.map((n) => (
            <Link
              key={n.href}
              href={n.href}
              itemProp="url"
              className="transition-colors hover:text-[var(--color-ink)]"
            >
              <span itemProp="name">{n.label}</span>
            </Link>
          ))}
        </nav>

        <div className="ml-auto flex items-center gap-3">
          <span className="hidden font-mono text-xs text-[var(--color-muted)] md:inline">
            v{summary.version}
          </span>
          <ThemeToggle />
          <a
            href={site.repo}
            className="rounded-md border px-2.5 py-1 font-mono text-xs text-[var(--color-muted)] transition-colors hover:border-[var(--color-key)] hover:text-[var(--color-ink)]"
          >
            GitHub
          </a>
        </div>
      </div>
    </header>
  );
}
