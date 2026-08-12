import type { Metadata } from "next";
import Link from "next/link";
import { guides, guidesByCategory } from "@/lib/guides";
import { site } from "@/lib/site";

export const metadata: Metadata = {
  title: "Guides for command output, JSON, and shell pipelines",
  description:
    "Practical jc-rs guides for NDJSON, JSONL, YAML, TOML, Linux logs, jq, Git output, curl headers, and reliable command-line parsing.",
  alternates: { canonical: "/guides" },
  openGraph: {
    title: "jc-rs guides",
    description: "Format decisions and tested command-line pipelines, written around the parsers that jc-rs actually ships.",
    url: `${site.origin}/guides`,
    images: [site.socialImage],
  },
  twitter: {
    card: "summary_large_image",
    title: "jc-rs guides",
    description: "Format decisions and tested command-line pipelines, written around the parsers that jc-rs actually ships.",
    images: [site.socialImage.url],
  },
};

const CATEGORY_COPY = {
  Formats: "What changes when the same records move between files, streams, and configuration formats.",
  Pipelines: "Commands you can run, inspect, and adapt without hiding the awkward cases.",
  Decisions: "Where a parser helps, where a native flag is better, and what tends to fail in production.",
} as const;

export default function GuidesIndex() {
  const schema = {
    "@context": "https://schema.org",
    "@type": "CollectionPage",
    name: "jc-rs guides",
    url: `${site.origin}/guides`,
    description: metadata.description,
    mainEntity: {
      "@type": "ItemList",
      itemListElement: guides.map((guide, index) => ({
        "@type": "ListItem",
        position: index + 1,
        url: `${site.origin}${guide.href}`,
        name: guide.title,
      })),
    },
  };

  return (
    <div className="mx-auto max-w-6xl px-5 py-14">
      <script
        type="application/ld+json"
        dangerouslySetInnerHTML={{ __html: JSON.stringify(schema).replaceAll("<", "\\u003c") }}
      />

      <nav className="font-mono text-xs text-[var(--color-muted)]" aria-label="Breadcrumb">
        <Link href="/" className="hover:text-[var(--color-ink)]">
          jc-rs
        </Link>
        <span className="text-[var(--color-faint)]"> / </span>
        <span aria-current="page">guides</span>
      </nav>

      <div className="mt-5 grid gap-8 border-b pb-12 lg:grid-cols-[minmax(0,1fr)_18rem] lg:items-end">
        <div>
          <p className="font-mono text-xs tracking-wide text-[var(--color-muted)] uppercase">
            Field notes
          </p>
          <h1 className="mt-3 max-w-3xl text-[clamp(2.5rem,6vw,4.5rem)]">
            From raw output to a pipeline you can trust.
          </h1>
        </div>
        <p className="max-w-xl text-[var(--color-muted)] lg:pb-1">
          These are working notes, not a tour of every parser. Each guide starts with a
          real format or shell problem, shows the exact boundary jc-rs handles, and links
          back to the parser behind the command.
        </p>
      </div>

      {guidesByCategory().map(({ category, items }) => (
        <section key={category} className="grid gap-6 border-b py-12 last:border-b-0 lg:grid-cols-[15rem_minmax(0,1fr)]">
          <div>
            <h2 className="text-2xl">{category}</h2>
            <p className="mt-2 max-w-xs text-sm text-[var(--color-muted)]">
              {CATEGORY_COPY[category]}
            </p>
          </div>

          <ol className="divide-y border-y">
            {items.map((guide) => (
              <li key={guide.href}>
                <Link
                  href={guide.href}
                  className="group grid gap-2 py-5 sm:grid-cols-[minmax(0,1fr)_auto] sm:gap-8"
                >
                  <span>
                    <span className="font-display text-xl font-semibold tracking-tight transition-colors group-hover:text-[var(--color-key)]">
                      {guide.title}
                    </span>
                    <span className="mt-1.5 block max-w-2xl text-sm leading-6 text-[var(--color-muted)]">
                      {guide.description}
                    </span>
                  </span>
                  <span className="self-center font-mono text-xs text-[var(--color-faint)] tabular-nums">
                    {guide.readingMinutes} min →
                  </span>
                </Link>
              </li>
            ))}
          </ol>
        </section>
      ))}
    </div>
  );
}
