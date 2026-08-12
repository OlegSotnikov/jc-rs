import type { Metadata } from "next";
import { ParserSearch } from "@/components/ParserSearch";
import { getParserSeo, parserSeoNames } from "@/lib/parser-seo";
import { getParser, grouped, slugOf, summary } from "@/lib/parsers";
import { site } from "@/lib/site";

const description = `Every one of the ${summary.documented} parsers jc-rs ships, grouped by domain, with the platforms each supports and the fixture coverage behind it.`;

export const metadata: Metadata = {
  title: "All parsers",
  description,
  alternates: { canonical: "/parsers" },
  openGraph: {
    siteName: site.name,
    type: "website",
    title: `All ${summary.documented} jc-rs parsers`,
    description,
    url: `${site.origin}/parsers`,
    images: [site.socialImage],
  },
  twitter: {
    card: "summary_large_image",
    title: `All ${summary.documented} jc-rs parsers`,
    description,
    images: [site.socialImage],
  },
};

export default function ParsersIndex() {
  const groups = grouped();

  return (
    <div className="mx-auto max-w-6xl px-5 py-14">
      <p className="font-mono text-xs tracking-wide text-[var(--color-muted)] uppercase">
        Reference
      </p>
      <h1 className="mt-3 text-4xl">{summary.documented} parsers</h1>
      <p className="mt-4 max-w-2xl text-lg text-[var(--color-muted)]">
        Grouped the way the crate is: by domain. {summary.withCoverage} are covered by the
        reference fixture corpus, {summary.streaming} stream line by line, and{" "}
        {summary.withExample} carry a worked example on their page.
      </p>

      <div className="mt-8">
        <ParserSearch
          items={groups.flatMap((g) =>
            g.items.map((p) => ({
              name: p.name,
              slug: slugOf(p),
              argument: p.argument,
              description: p.description,
              group: g.title,
            })),
          )}
        />
      </div>

      <section className="mt-12 border-y py-8">
        <div className="flex flex-wrap items-end justify-between gap-4">
          <div>
            <h2 className="text-2xl">Use a parser in this tab</h2>
            <p className="mt-2 max-w-2xl text-sm text-[var(--color-muted)]">
              These pages load the Rust parser as WebAssembly. Paste or open text, inspect the
              resulting JSON, then take the same command to your shell.
            </p>
          </div>
          <span className="font-mono text-[11px] text-[var(--color-str)]">local · no input upload</span>
        </div>

        <ul className="mt-6 grid gap-x-8 sm:grid-cols-2 lg:grid-cols-3">
          {parserSeoNames.map((name) => {
            const parser = getParser(name)!;
            const seo = getParserSeo(name)!;
            return (
              <li key={name} className="border-t first:border-t-0 sm:first:border-t">
                <a href={`/parsers/${slugOf(parser)}`} className="group block py-3.5">
                  <span className="font-display font-semibold transition-colors group-hover:text-[var(--color-key)]">
                    {seo.title}
                  </span>
                  <span className="mt-1 block line-clamp-2 text-xs leading-5 text-[var(--color-muted)]">
                    {seo.description}
                  </span>
                </a>
              </li>
            );
          })}
        </ul>
      </section>

      {groups.map((g) => (
        <section
          key={g.title}
          id={g.title.toLowerCase().replace(/[^a-z]+/g, "-")}
          className="mt-14 scroll-mt-20"
        >
          <div className="flex items-baseline gap-3 border-b pb-3">
            <h2 className="text-2xl">{g.title}</h2>
            <span className="font-mono text-xs text-[var(--color-faint)] tabular-nums">
              {g.items.length}
            </span>
            <span className="ml-auto text-sm text-[var(--color-muted)]">{g.blurb}</span>
          </div>

          <ul className="mt-5 grid gap-2 sm:grid-cols-2 lg:grid-cols-3">
            {g.items.map((p) => (
              <li key={p.name}>
                <a
                  href={`/parsers/${slugOf(p)}`}
                  className="block rounded-lg border bg-[var(--color-surface)] px-4 py-3 transition-colors hover:border-[var(--color-key)]"
                >
                  <div className="flex items-baseline gap-2">
                    <code className="font-mono text-sm font-medium">{p.name}</code>
                    {p.streaming && (
                      <span className="rounded-sm bg-[color-mix(in_oklab,var(--color-str)_18%,transparent)] px-1.5 font-mono text-[10px] text-[var(--color-str)]">
                        stream
                      </span>
                    )}
                    {p.hidden && (
                      <span className="font-mono text-[10px] text-[var(--color-faint)]">
                        via --proc
                      </span>
                    )}
                  </div>
                  <p className="mt-1 line-clamp-2 text-xs text-[var(--color-muted)]">
                    {p.description}
                  </p>
                </a>
              </li>
            ))}
          </ul>
        </section>
      ))}
    </div>
  );
}
