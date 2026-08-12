import type { Metadata } from "next";
import Link from "next/link";
import { site } from "@/lib/site";

const canonical = "/guides/ascii-table-to-json";
const title = "Convert an ASCII table to JSON on the command line";
const description =
  "Convert ASCII or Unicode tables to JSON with jc-rs. Covers fixed-width columns, multiline rows, normalized headers, jq, and layouts that are unsafe to parse.";
const published = "2026-08-11";

export const metadata: Metadata = {
  title,
  description,
  alternates: { canonical },
  openGraph: {
    siteName: site.name,
    type: "article",
    title,
    description,
    url: `${site.origin}${canonical}`,
    publishedTime: published,
    modifiedTime: published,
    authors: [site.authorUrl],
    images: [site.socialImage],
  },
  twitter: { card: "summary_large_image", title, description, images: [site.socialImage] },
};

const articleJsonLd = {
  "@context": "https://schema.org",
  "@type": "Article",
  headline: title,
  description,
  datePublished: published,
  dateModified: published,
  mainEntityOfPage: `${site.origin}${canonical}`,
  articleSection: "Command-line guides",
  author: { "@type": "Person", name: site.author, url: site.authorUrl },
  publisher: { "@type": "Organization", name: site.name, url: site.origin },
  about: [
    "ASCII table to JSON",
    "table to JSON",
    "text table parser",
    "Unicode table",
  ],
};

const simpleTable = `NAME        STATUS     PORT
api         healthy    8080
worker      draining   9090`;

const multilineTable = `+----------+--------+--------------------+
| SERVICE  | OWNER  | NOTE               |
+==========+========+====================+
| api      | ops    | waiting for        |
|          |        | database migration |
+----------+--------+--------------------+
| worker   | data   | ready              |
+----------+--------+--------------------+`;

export default function AsciiTableToJsonGuide() {
  return (
    <article className="mx-auto max-w-5xl px-5 py-14">
      <script
        type="application/ld+json"
        dangerouslySetInnerHTML={{
          __html: JSON.stringify(articleJsonLd).replaceAll("<", "\\u003c"),
        }}
      />

      <nav
        aria-label="Breadcrumb"
        className="font-mono text-xs text-[var(--color-muted)]"
      >
        <Link href="/guides" className="hover:text-[var(--color-ink)]">
          guides
        </Link>
        <span className="text-[var(--color-faint)]"> / </span>
        <span>ASCII table to JSON</span>
      </nav>

      <header className="mt-5 max-w-3xl">
        <p className="font-mono text-xs tracking-wide text-[var(--color-muted)] uppercase">
          Table conversion guide
        </p>
        <h1 className="mt-3 text-4xl sm:text-5xl">
          Convert an ASCII table to JSON on the command line
        </h1>
        <p className="mt-5 text-lg leading-relaxed text-[var(--color-muted)]">
          An aligned text table often exposes enough structure to recover
          records: headers name the columns, while spacing or borders mark their
          boundaries. jc-rs handles simple fixed-width output and bordered
          tables whose logical rows span several lines. Arbitrary prose does not
          provide those guarantees.
        </p>
        <p className="mt-5 text-sm text-[var(--color-faint)]">
          By{" "}
          <a
            href={site.authorUrl}
            rel="author"
            className="text-[var(--color-key)] underline-offset-4 hover:underline"
          >
            {site.author}
          </a>{" "}
          · Published <time dateTime={published}>August 11, 2026</time> · Input
          and output examples checked against both jc-rs ASCII table parsers
        </p>
      </header>

      <div
        aria-label="The table conversion pipeline"
        className="mt-9 grid overflow-hidden rounded-xl border bg-[var(--color-surface)] sm:grid-cols-[1fr_auto_1fr_auto_1fr_auto_1fr]"
      >
        <Stage label="Input" value="aligned table" />
        <Arrow />
        <Stage label="Parse" value="jc-rs" accent />
        <Arrow />
        <Stage label="Shape" value="JSON rows" />
        <Arrow />
        <Stage label="Query" value="jq" />
      </div>

      <nav
        aria-label="On this page"
        className="mt-9 rounded-xl border bg-[var(--color-surface)] p-5"
      >
        <p className="font-mono text-[11px] tracking-wide text-[var(--color-faint)] uppercase">
          On this page
        </p>
        <div className="mt-3 grid gap-x-8 gap-y-2 text-sm sm:grid-cols-2">
          <a href="#choose" className="hover:text-[var(--color-key)]">
            Choose simple or multiline
          </a>
          <a href="#simple" className="hover:text-[var(--color-key)]">
            Convert a simple table
          </a>
          <a href="#multiline" className="hover:text-[var(--color-key)]">
            Preserve multiline cells
          </a>
          <a href="#headers" className="hover:text-[var(--color-key)]">
            Header and value behavior
          </a>
          <a href="#limits" className="hover:text-[var(--color-key)]">
            Know what is not a table
          </a>
          <a href="#troubleshoot" className="hover:text-[var(--color-key)]">
            Diagnose bad boundaries
          </a>
        </div>
      </nav>

      <div className="mt-14 max-w-3xl space-y-16">
        <section id="choose" className="scroll-mt-24">
          <h2 className="text-3xl">
            Choose by row structure, not border style
          </h2>
          <p className="mt-4 leading-relaxed text-[var(--color-muted)]">
            jc-rs has two related parsers. Both accept ASCII or Unicode table
            characters and strip terminal ANSI color sequences. Choose between
            them based on whether one data row fits on one physical line.
          </p>
          <div className="mt-6 overflow-hidden rounded-xl border bg-[var(--color-surface)]">
            <ChoiceRow
              input="One physical line per record"
              parser="--asciitable"
              href="/parsers/asciitable"
              note="Simple, Markdown-like, or bordered column tables"
            />
            <ChoiceRow
              input="One logical record spans lines"
              parser="--asciitable-m"
              href="/parsers/asciitable-m"
              note="Pretty bordered tables with separators between rows"
            />
            <ChoiceRow
              input="A reliable delimiter exists"
              parser="--csv / --tsv"
              href="/parsers/csv"
              note="Prefer the explicit delimiter over visual alignment"
            />
            <ChoiceRow
              input="The source already offers JSON"
              parser="native output"
              note="Keep the native schema; no table parser is needed"
            />
          </div>
        </section>

        <section id="simple" className="scroll-mt-24">
          <h2 className="text-3xl">
            Simple table: one line becomes one object
          </h2>
          <p className="mt-4 leading-relaxed text-[var(--color-muted)]">
            The first non-empty row supplies the headers. Spacing in that row
            establishes column positions; the following rows are read against
            those positions.
          </p>
          <CodeBlock label="table.txt">{simpleTable}</CodeBlock>
          <CodeBlock label="Bash">
            {`jc-rs -p --asciitable < table.txt`}
          </CodeBlock>
          <CodeBlock label="JSON">
            {`[
  {
    "name": "api",
    "port": "8080",
    "status": "healthy"
  },
  {
    "name": "worker",
    "port": "9090",
    "status": "draining"
  }
]`}
          </CodeBlock>
          <p className="mt-4 leading-relaxed text-[var(--color-muted)]">
            The output is an array, so jq can filter it without returning to
            column offsets. This example converts the port explicitly because
            the table parser preserves it as text:
          </p>
          <CodeBlock label="Filter the JSON rows">
            {`jc-rs --asciitable < table.txt |
  jq 'map(select((.port | tonumber) >= 9000))'`}
          </CodeBlock>
        </section>

        <section id="multiline" className="scroll-mt-24">
          <h2 className="text-3xl">
            Multiline table: preserve wrapped cell content
          </h2>
          <p className="mt-4 leading-relaxed text-[var(--color-muted)]">
            In a pretty table, border rows identify where one logical record
            ends and the next starts. The multiline parser joins successive
            physical lines in the same cell with a newline.
          </p>
          <CodeBlock label="multiline-table.txt">{multilineTable}</CodeBlock>
          <CodeBlock label="Bash">
            {`jc-rs -p --asciitable-m < multiline-table.txt`}
          </CodeBlock>
          <CodeBlock label="JSON">
            {`[
  {
    "note": "waiting for\\ndatabase migration",
    "owner": "ops",
    "service": "api"
  },
  {
    "note": "ready",
    "owner": "data",
    "service": "worker"
  }
]`}
          </CodeBlock>
          <p className="mt-4 text-sm leading-relaxed text-[var(--color-faint)]">
            The multiline parser intentionally accepts “pretty” bordered tables.
            If it detects a simple or Markdown table, it returns an error and
            directs you to <code className="font-mono">--asciitable</code>; that
            prevents ordinary rows from being collapsed together by guesswork.
          </p>
        </section>

        <section id="headers" className="scroll-mt-24">
          <h2 className="text-3xl">How headers become JSON keys</h2>
          <p className="mt-4 leading-relaxed text-[var(--color-muted)]">
            Headers are lowercased and normalized toward snake_case. Spaces
            between words become underscores, punctuation is normalized, and an
            empty cell becomes null. Multiline header bands are collapsed into
            combined names.
          </p>
          <div className="mt-6 overflow-hidden rounded-xl border bg-[var(--color-surface)]">
            <HeaderRow source="Service Name" json="service_name" />
            <HeaderRow source="PORT" json="port" />
            <HeaderRow
              source="two header bands: Disk / Used"
              json="disk_used"
            />
            <HeaderRow source="empty data cell" json="null" />
          </div>
          <p className="mt-5 leading-relaxed text-[var(--color-muted)]">
            Do not assume a numeric-looking cell became a JSON number. Inspect
            the actual result and use jq&apos;s{" "}
            <code className="font-mono text-sm">tonumber</code> at the point
            where numeric semantics are required. This also makes conversion
            failures visible.
          </p>
        </section>

        <section id="limits" className="scroll-mt-24">
          <h2 className="text-3xl">
            “Text to JSON” only works when the text has a schema
          </h2>
          <p className="mt-4 leading-relaxed text-[var(--color-muted)]">
            A table parser can recover explicit columns. It cannot know whether
            a sentence, stack trace, paragraph, or casually spaced note contains
            a name, timestamp, status, or message. Converting arbitrary prose
            would require domain rules, not a universal parser.
          </p>
          <div className="mt-6 grid gap-4 sm:grid-cols-2">
            <BoundaryCard title="Good table input" tone="good">
              A header row, repeatable column positions, and one clear record
              boundary.
            </BoundaryCard>
            <BoundaryCard title="Not enough structure" tone="warn">
              Wrapped prose, inconsistent labels, or whitespace that changes
              meaning from line to line.
            </BoundaryCard>
          </div>
          <p className="mt-5 leading-relaxed text-[var(--color-muted)]">
            If the source is delimited, use the{" "}
            <Link
              href="/parsers/csv"
              className="text-[var(--color-key)] underline-offset-4 hover:underline"
            >
              CSV parser
            </Link>{" "}
            or{" "}
            <Link
              href="/parsers/tsv"
              className="text-[var(--color-key)] underline-offset-4 hover:underline"
            >
              TSV parser
            </Link>
            . If each line is genuinely key/value data, inspect the{" "}
            <Link
              href="/parsers/kv"
              className="text-[var(--color-key)] underline-offset-4 hover:underline"
            >
              key/value parser
            </Link>
            . Pick the grammar the producer actually writes.
          </p>
        </section>

        <section id="troubleshoot" className="scroll-mt-24">
          <h2 className="text-3xl">When columns land in the wrong field</h2>
          <ol className="mt-5 space-y-5 text-[var(--color-muted)]">
            <li className="grid grid-cols-[2rem_1fr] gap-3">
              <span className="font-mono text-sm text-[var(--color-key)]">
                01
              </span>
              <span>
                Capture raw output rather than copying from a rendered web page
                or proportional font. Character positions are part of the input.
              </span>
            </li>
            <li className="grid grid-cols-[2rem_1fr] gap-3">
              <span className="font-mono text-sm text-[var(--color-key)]">
                02
              </span>
              <span>
                Make the producer use a wide, non-interactive layout when
                possible. Terminal-width wrapping can turn one record into
                several unrelated lines.
              </span>
            </li>
            <li className="grid grid-cols-[2rem_1fr] gap-3">
              <span className="font-mono text-sm text-[var(--color-key)]">
                03
              </span>
              <span>
                Check the header first. A header centered differently from its
                data may establish the wrong boundary even when the rows look
                aligned to a person.
              </span>
            </li>
            <li className="grid grid-cols-[2rem_1fr] gap-3">
              <span className="font-mono text-sm text-[var(--color-key)]">
                04
              </span>
              <span>
                Use <code className="font-mono text-sm">--asciitable-m</code>{" "}
                only when explicit border rows delimit logical records.
                Otherwise keep the simple parser.
              </span>
            </li>
            <li className="grid grid-cols-[2rem_1fr] gap-3">
              <span className="font-mono text-sm text-[var(--color-key)]">
                05
              </span>
              <span>
                Compare several output versions. If a command changes its layout
                across systems, prefer that command&apos;s dedicated jc-rs
                parser when one exists.
              </span>
            </li>
          </ol>
        </section>

        <section className="rounded-xl border bg-[var(--color-surface)] p-6">
          <h2 className="text-2xl">
            Choose the most specific parser available
          </h2>
          <p className="mt-3 text-sm leading-relaxed text-[var(--color-muted)]">
            The parser references document each input contract. The Bash and jq
            guide explains how to carry the resulting rows safely into a larger
            script.
          </p>
          <div className="mt-5 grid gap-3 sm:grid-cols-2">
            <RelatedLink
              href="/parsers/asciitable"
              title="Simple ASCII table parser"
            >
              One line per record, ASCII or Unicode borders, normalized headers.
            </RelatedLink>
            <RelatedLink
              href="/parsers/asciitable-m"
              title="Multiline table parser"
            >
              Pretty tables with wrapped headers or cells.
            </RelatedLink>
            <RelatedLink
              href="/guides/bash-jc-rs-jq"
              title="Bash, jc-rs and jq"
            >
              Quote filters, preserve failures, and validate empty results.
            </RelatedLink>
            <RelatedLink href="/parsers" title="Browse dedicated parsers">
              A command-specific schema is preferable when one is available.
            </RelatedLink>
          </div>
          <Link
            href="/install"
            className="mt-5 inline-block rounded-md border px-4 py-2 text-sm transition-colors hover:border-[var(--color-key)]"
          >
            Install jc-rs
          </Link>
        </section>
      </div>
    </article>
  );
}

function Stage({
  label,
  value,
  accent = false,
}: {
  label: string;
  value: string;
  accent?: boolean;
}) {
  return (
    <div className="px-4 py-4">
      <span className="block font-mono text-[10px] tracking-wide text-[var(--color-faint)] uppercase">
        {label}
      </span>
      <span
        className={`mt-1 block font-mono text-sm ${accent ? "text-[var(--color-key)]" : ""}`}
      >
        {value}
      </span>
    </div>
  );
}

function Arrow() {
  return (
    <span
      aria-hidden="true"
      className="hidden self-center font-mono text-[var(--color-faint)] sm:block"
    >
      →
    </span>
  );
}

function ChoiceRow({
  input,
  parser,
  href,
  note,
}: {
  input: string;
  parser: string;
  href?: string;
  note: string;
}) {
  const parserLabel = href ? (
    <Link
      href={href}
      className="font-mono text-xs text-[var(--color-key)] hover:underline"
    >
      {parser}
    </Link>
  ) : (
    <code className="font-mono text-xs text-[var(--color-str)]">{parser}</code>
  );

  return (
    <div className="grid gap-2 border-b px-5 py-4 last:border-b-0 sm:grid-cols-[12rem_8rem_1fr]">
      <p className="text-sm font-medium">{input}</p>
      <div>{parserLabel}</div>
      <p className="text-sm text-[var(--color-muted)]">{note}</p>
    </div>
  );
}

function HeaderRow({ source, json }: { source: string; json: string }) {
  return (
    <div className="grid grid-cols-[minmax(0,1fr)_auto_minmax(0,1fr)] items-center gap-4 border-b px-5 py-4 last:border-b-0">
      <code className="font-mono text-xs">{source}</code>
      <span aria-hidden="true" className="text-[var(--color-faint)]">
        →
      </span>
      <code className="font-mono text-xs text-[var(--color-key)]">{json}</code>
    </div>
  );
}

function BoundaryCard({
  title,
  tone,
  children,
}: {
  title: string;
  tone: "good" | "warn";
  children: React.ReactNode;
}) {
  return (
    <div className="rounded-xl border bg-[var(--color-surface)] p-5">
      <p
        className={`font-mono text-xs ${tone === "good" ? "text-[var(--color-str)]" : "text-[var(--color-num)]"}`}
      >
        {title}
      </p>
      <p className="mt-3 text-sm leading-relaxed text-[var(--color-muted)]">
        {children}
      </p>
    </div>
  );
}

function CodeBlock({
  label,
  children,
}: {
  label: string;
  children: React.ReactNode;
}) {
  return (
    <figure className="mt-5 overflow-hidden rounded-xl border bg-[var(--color-sunk)]">
      <figcaption className="border-b bg-[var(--color-surface)] px-4 py-2 font-mono text-[10px] tracking-wide text-[var(--color-faint)] uppercase">
        {label}
      </figcaption>
      <pre className="overflow-x-auto p-4 font-mono text-xs leading-relaxed">
        <code>{children}</code>
      </pre>
    </figure>
  );
}

function RelatedLink({
  href,
  title,
  children,
}: {
  href: string;
  title: string;
  children: React.ReactNode;
}) {
  return (
    <Link
      href={href}
      className="rounded-lg border p-4 transition-colors hover:border-[var(--color-key)]"
    >
      <span className="block font-display font-semibold">{title}</span>
      <span className="mt-1 block text-xs leading-relaxed text-[var(--color-muted)]">
        {children}
      </span>
    </Link>
  );
}
