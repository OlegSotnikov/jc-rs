import type { Metadata } from "next";
import { ParserSearch } from "@/components/ParserSearch";
import { grouped, slugOf, summary } from "@/lib/parsers";

export const metadata: Metadata = {
  title: "All parsers",
  description: `Every one of the ${summary.documented} parsers jc-rs ships, grouped by domain, with the platforms each supports and the fixture coverage behind it.`,
  alternates: { canonical: "/parsers" },
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
        Grouped the way the crate is: by domain. {summary.withCoverage} of them are covered by
        jc&apos;s fixture corpus, {summary.streaming} stream line by line, and{" "}
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
