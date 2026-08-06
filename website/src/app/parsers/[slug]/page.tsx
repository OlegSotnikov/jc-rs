import Link from "next/link";
import type { Metadata } from "next";
import { notFound } from "next/navigation";
import { Panes } from "@/components/Panes";
import { CopyLine } from "@/components/CopyLine";
import { fromSlug, parsers, platformLabel, slugOf, summary } from "@/lib/parsers";
import { site } from "@/lib/site";

type Params = { params: Promise<{ slug: string }> };

export function generateStaticParams() {
  return parsers.map((p) => ({ slug: slugOf(p) }));
}

export async function generateMetadata({ params }: Params): Promise<Metadata> {
  const { slug } = await params;
  const p = fromSlug(slug);
  if (!p) return {};

  const where = p.magic[0] ? `\`${p.magic[0]}\`` : p.name;
  return {
    title: `${p.name} — ${p.description}`,
    description:
      `${p.description}. Pipe ${where} into jc-rs ${p.argument} and get structured JSON` +
      (p.coverage ? `, verified against ${p.coverage.tested} reference fixtures.` : "."),
    alternates: { canonical: `/parsers/${slug}` },
    openGraph: {
      title: `jc-rs ${p.argument}`,
      description: p.description,
      url: `${site.origin}/parsers/${slug}`,
    },
  };
}

export default async function ParserPage({ params }: Params) {
  const { slug } = await params;
  const p = fromSlug(slug);
  if (!p) notFound();

  const command = p.magic[0] ?? p.name;
  const pipeline = p.streaming
    ? `${command} | jc-rs -u ${p.argument}`
    : `${command} | jc-rs ${p.argument}`;

  return (
    <div className="mx-auto max-w-5xl px-5 py-14">
      <nav className="font-mono text-xs text-[var(--color-muted)]">
        <Link href="/parsers" className="hover:text-[var(--color-ink)]">
          parsers
        </Link>
        <span className="text-[var(--color-faint)]"> / </span>
        <span>{p.name}</span>
      </nav>

      <div className="mt-4 flex flex-wrap items-baseline gap-x-4 gap-y-2">
        <h1 className="font-mono text-4xl font-semibold tracking-tight">{p.name}</h1>
        <code className="font-mono text-sm text-[var(--color-key)]">{p.argument}</code>
        <span className="font-mono text-xs text-[var(--color-faint)]">v{p.version}</span>
      </div>
      <p className="mt-3 max-w-2xl text-lg text-[var(--color-muted)]">{p.description}</p>

      <div className="mt-5 flex flex-wrap gap-1.5">
        {p.platforms.map((pl) => (
          <Chip key={pl}>{platformLabel(pl)}</Chip>
        ))}
        {p.tags.map((t) => (
          <Chip key={t} tone="muted">
            {t.toLowerCase()}
          </Chip>
        ))}
        {p.streaming && <Chip tone="str">streaming</Chip>}
        {p.deprecated && <Chip tone="num">deprecated</Chip>}
      </div>

      <div className="mt-8 max-w-2xl">
        <CopyLine command={pipeline} />
      </div>

      {p.example ? (
        <section className="mt-12">
          <h2 className="text-2xl">A real pair</h2>
          <p className="mt-2 max-w-2xl text-sm text-[var(--color-muted)]">
            From the reference corpus (
            <code className="font-mono">{p.example.fixture}</code>). The left pane is the input
            the fixture ships; the right is what jc-rs writes for it, compared byte for byte
            against the expected output on every commit. Hover a value to see where it came
            from.
          </p>
          <div className="mt-5">
            <Panes
              input={p.example.input}
              output={p.example.output}
              inputLabel={
                <>
                  <span>{command}</span>
                  <span className="normal-case">{p.example.platform}</span>
                </>
              }
              outputLabel={
                <>
                  <span>jc-rs {p.argument}</span>
                  <span className="normal-case">json</span>
                </>
              }
            />
          </div>
        </section>
      ) : (
        <section className="mt-12 rounded-xl border bg-[var(--color-surface)] p-5">
          <h2 className="text-lg">No worked example here</h2>
          <p className="mt-2 max-w-2xl text-sm text-[var(--color-muted)]">
            The reference corpus ships no fixture pair for this parser small enough to read
            on a page, so rather than invent one this page shows none. Run it against your own
            output.
          </p>
        </section>
      )}

      <section className="mt-12 grid gap-4 sm:grid-cols-2">
        <Fact title="Fixture coverage">
          {p.coverage ? (
            <>
              <span className="font-mono text-2xl tabular-nums">
                {p.coverage.match}/{p.coverage.tested}
              </span>
              <span className="mt-1 block text-sm text-[var(--color-muted)]">
                oracle-valid reference pairs match byte for byte
              </span>
            </>
          ) : (
            <>
              <span className="font-mono text-2xl">none</span>
              <span className="mt-1 block text-sm text-[var(--color-muted)]">
                The reference corpus ships no oracle-valid fixture for this parser, so the
                differential cannot speak for it. That is reported, not hidden.
              </span>
            </>
          )}
        </Fact>

        <Fact title="Magic syntax">
          {p.magic.length ? (
            <>
              <code className="font-mono text-sm">jc-rs {p.magic[0]}</code>
              <span className="mt-1 block text-sm text-[var(--color-muted)]">
                jc-rs runs the command itself and parses what it prints. Recognised for:{" "}
                {p.magic.map((m) => `\`${m}\``).join(", ")}
              </span>
            </>
          ) : (
            <>
              <span className="font-mono text-sm">not available</span>
              <span className="mt-1 block text-sm text-[var(--color-muted)]">
                This parser reads a file or a string rather than a command, so there is
                nothing for jc-rs to run. Pipe it in.
              </span>
            </>
          )}
        </Fact>
      </section>

      <p className="mt-10 text-sm text-[var(--color-faint)]">
        {p.source && (
          <>
            Source:{" "}
            <a
              href={`${site.repo}/blob/master/${p.source}`}
              className="font-mono text-[var(--color-key)] underline-offset-4 hover:underline"
            >
              {p.source}
            </a>
            .{" "}
          </>
        )}
        The JSON here is schema-compatible with the original Python tool, so anything already
        reading it keeps working.{" "}
        <Link href="/compare" className="text-[var(--color-key)] underline-offset-4 hover:underline">
          Compare
        </Link>
      </p>
    </div>
  );
}

function Chip({
  children,
  tone = "key",
}: {
  children: React.ReactNode;
  tone?: "key" | "muted" | "str" | "num";
}) {
  const tint =
    tone === "muted"
      ? "text-[var(--color-muted)] border-[var(--color-rule)]"
      : `text-[var(--color-${tone})] border-[color-mix(in_oklab,var(--color-${tone})_40%,transparent)]`;
  return (
    <span className={`rounded-full border px-2.5 py-0.5 font-mono text-[11px] ${tint}`}>
      {children}
    </span>
  );
}

function Fact({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <div className="rounded-xl border bg-[var(--color-surface)] p-5">
      <p className="font-mono text-[11px] tracking-wide text-[var(--color-faint)] uppercase">
        {title}
      </p>
      <div className="mt-3">{children}</div>
    </div>
  );
}
