import type { Metadata } from "next";
import Link from "next/link";
import type { ReactNode } from "react";
import { site } from "@/lib/site";

const slug = "/guides/ndjson-vs-json-vs-jsonl";
const pageUrl = site.origin + slug;
const published = "2026-08-11";

export const metadata: Metadata = {
  title: "NDJSON vs JSON vs JSONL: Practical Streaming Guide",
  description:
    "Learn the difference between JSON, NDJSON, JSONL and JSON Lines, when line-delimited data wins, and how to stream or convert it safely.",
  alternates: { canonical: slug },
  openGraph: {
    type: "article",
    title: "NDJSON vs JSON vs JSONL: a practical streaming guide",
    description:
      "One document or one record per line? Compare the formats, trade-offs, conversion commands and real jc-rs streaming behavior.",
    url: pageUrl,
    publishedTime: published,
    modifiedTime: published,
    authors: [site.authorUrl],
    images: [site.socialImage],
  },
  twitter: {
    card: "summary_large_image",
    title: "NDJSON vs JSON vs JSONL: a practical streaming guide",
    description:
      "One document or one record per line? Compare the formats, trade-offs, conversion commands and real jc-rs streaming behavior.",
    images: [site.socialImage.url],
  },
};

const articleJsonLd = {
  "@context": "https://schema.org",
  "@type": "Article",
  headline:
    "NDJSON vs JSON vs JSONL: the practical guide for streaming command output",
  description:
    "A practical comparison of JSON documents, NDJSON, JSONL and JSON Lines, including streaming behavior, failure recovery and command-line conversion.",
  mainEntityOfPage: pageUrl,
  url: pageUrl,
  datePublished: published,
  dateModified: published,
  author: {
    "@type": "Person",
    name: site.author,
    url: site.authorUrl,
  },
  publisher: {
    "@type": "Organization",
    name: site.name,
    url: site.origin,
  },
  about: ["NDJSON", "JSONL", "JSON Lines", "JSON", "stream processing"],
};

const decisionRows = [
  [
    "A bounded API response",
    "JSON",
    "One complete payload is easy to validate and consume.",
  ],
  [
    "A live log or event feed",
    "NDJSON / JSONL",
    "A consumer can act whenever the next line arrives.",
  ],
  [
    "A large export processed record by record",
    "NDJSON / JSONL",
    "Line boundaries make sequential processing straightforward.",
  ],
  [
    "One deeply nested configuration object",
    "JSON",
    "The document is one value, not a sequence of independent records.",
  ],
  [
    "A small file people inspect by hand",
    "Pretty JSON",
    "Indentation helps more than line framing at this size.",
  ],
  [
    "Random lookup by record ID",
    "Neither by itself",
    "Add an index or use a database; a file extension does not create random access.",
  ],
] as const;

export default function NdjsonGuide() {
  return (
    <div className="mx-auto max-w-5xl px-5 py-14">
      <script
        type="application/ld+json"
        dangerouslySetInnerHTML={{
          __html: JSON.stringify(articleJsonLd).replace(/</g, "\\u003c"),
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
        <span>ndjson, json and jsonl</span>
      </nav>

      <article>
        <header className="mt-5">
          <p className="font-mono text-xs tracking-wide text-[var(--color-muted)] uppercase">
            Format guide
          </p>
          <h1 className="mt-3 max-w-4xl text-4xl sm:text-5xl">
            NDJSON vs JSON vs JSONL
          </h1>
          <p className="mt-5 max-w-3xl text-lg leading-8 text-[var(--color-muted)]">
            JSON is one complete document. NDJSON is a sequence of complete JSON
            values, one per physical line. JSONL and JSON Lines usually mean the
            same line-delimited layout. Use JSON for a bounded payload; use
            NDJSON when records arrive, move, or fail one at a time.
          </p>
          <div className="mt-5 flex flex-wrap items-center gap-x-3 gap-y-1 font-mono text-xs text-[var(--color-faint)]">
            <a href={site.authorUrl} className="hover:text-[var(--color-ink)]">
              {site.author}
            </a>
            <span aria-hidden="true">·</span>
            <time dateTime={published}>August 11, 2026</time>
            <span aria-hidden="true">·</span>
            <span>9 min read</span>
          </div>
        </header>

        <section aria-labelledby="same-records" className="mt-10">
          <div className="rounded-xl border bg-[var(--color-surface)] p-5 sm:p-6">
            <div className="flex flex-wrap items-baseline justify-between gap-2">
              <h2 id="same-records" className="text-xl">
                Same records, different boundary
              </h2>
              <p className="font-mono text-[11px] tracking-wide text-[var(--color-faint)] uppercase">
                comma + brackets vs newline
              </p>
            </div>
            <div className="mt-5 grid gap-4 lg:grid-cols-2">
              <CodeSample label="records.json · one JSON array">
                {`[
  {"host":"web-1","status":200},
  {"host":"web-2","status":503},
  {"host":"web-3","status":200}
]`}
              </CodeSample>
              <CodeSample label="records.ndjson · three JSON values">
                {`{"host":"web-1","status":200}
{"host":"web-2","status":503}
{"host":"web-3","status":200}`}
              </CodeSample>
            </div>
            <p className="mt-4 max-w-3xl text-sm leading-6 text-[var(--color-muted)]">
              The objects are ordinary JSON in both files. Only the outer
              framing changes. In NDJSON, the newline closes the record, so a
              record cannot be spread across several display lines.
            </p>
          </div>
        </section>

        <div className="mt-14 grid gap-10 lg:grid-cols-[13rem_minmax(0,1fr)] lg:gap-14">
          <aside className="self-start lg:sticky lg:top-24">
            <p className="font-mono text-[11px] tracking-wide text-[var(--color-faint)] uppercase">
              In this guide
            </p>
            <ol className="mt-3 space-y-2 text-sm text-[var(--color-muted)]">
              <li>
                <a
                  href="#definitions"
                  className="hover:text-[var(--color-ink)]"
                >
                  Names and structure
                </a>
              </li>
              <li>
                <a href="#choose" className="hover:text-[var(--color-ink)]">
                  Which one to choose
                </a>
              </li>
              <li>
                <a href="#tradeoffs" className="hover:text-[var(--color-ink)]">
                  Operational trade-offs
                </a>
              </li>
              <li>
                <a href="#jc-rs" className="hover:text-[var(--color-ink)]">
                  Streaming with jc-rs
                </a>
              </li>
              <li>
                <a href="#convert" className="hover:text-[var(--color-ink)]">
                  Convert JSON and JSONL
                </a>
              </li>
              <li>
                <a href="#faq" className="hover:text-[var(--color-ink)]">
                  Common questions
                </a>
              </li>
            </ol>
          </aside>

          <div className="min-w-0 max-w-3xl">
            <section id="definitions" className="scroll-mt-24">
              <h2 className="text-3xl">Definitions and record boundaries</h2>

              <h3 className="mt-8 text-xl">
                JSON: one value owns the whole document
              </h3>
              <p className="mt-3 leading-7 text-[var(--color-muted)]">
                A JSON document contains one top-level value: an object, array,
                string, number, boolean, or null. Whitespace around and inside
                that value is insignificant. A newline therefore has no special
                record meaning in ordinary JSON; it may simply be indentation
                inside one large array.
              </p>
              <p className="mt-3 leading-7 text-[var(--color-muted)]">
                An array is the usual way to package several records. That is an
                excellent shape for a finite HTTP response or a file that will
                be read as a unit. It is less handy for a feed whose closing
                bracket may not arrive for hours.
              </p>

              <h3 className="mt-8 text-xl">NDJSON: newline-delimited JSON</h3>
              <p className="mt-3 leading-7 text-[var(--color-muted)]">
                NDJSON adds a framing rule to JSON: every non-empty line is one
                complete JSON value. A consumer can read a line, parse it, and
                release it without waiting for an outer array to close. Newlines
                inside a JSON string remain escaped as{" "}
                <code className="font-mono text-sm">\n</code>; they are not
                literal line breaks in the file.
              </p>
              <p className="mt-3 leading-7 text-[var(--color-muted)]">
                Objects are the normal record shape, but the JSON Lines model
                permits any valid JSON value on a line. In practice, agree on
                one shape and schema. A stream that unexpectedly mixes objects,
                arrays, and scalars is valid syntax but awkward data.
              </p>

              <h3 className="mt-8 text-xl">
                JSONL and JSON Lines: another name, not another shape
              </h3>
              <p className="mt-3 leading-7 text-[var(--color-muted)]">
                In everyday tooling,{" "}
                <strong className="font-medium text-[var(--color-ink)]">
                  JSONL
                </strong>
                ,{" "}
                <strong className="font-medium text-[var(--color-ink)]">
                  JSON Lines
                </strong>
                , and{" "}
                <strong className="font-medium text-[var(--color-ink)]">
                  NDJSON
                </strong>{" "}
                describe the same useful contract: one JSON value per line. The
                common extensions are{" "}
                <code className="font-mono text-sm">.jsonl</code> and{" "}
                <code className="font-mono text-sm">.ndjson</code>. A particular
                API may prescribe one name or media type, so follow that API at
                the boundary; do not rewrite the bytes merely to change the
                label.
              </p>
            </section>

            <section id="choose" className="mt-14 scroll-mt-24">
              <h2 className="text-3xl">
                JSON or NDJSON: a working decision table
              </h2>
              <p className="mt-3 leading-7 text-[var(--color-muted)]">
                Choose based on how the data moves. The same record schema can
                travel in either container.
              </p>
              <div className="mt-6 overflow-x-auto rounded-xl border bg-[var(--color-surface)]">
                <table className="w-full min-w-[42rem] border-collapse text-left text-sm">
                  <thead className="bg-[var(--color-sunk)] font-mono text-[11px] tracking-wide text-[var(--color-faint)] uppercase">
                    <tr>
                      <th scope="col" className="px-4 py-3 font-medium">
                        Situation
                      </th>
                      <th scope="col" className="px-4 py-3 font-medium">
                        Prefer
                      </th>
                      <th scope="col" className="px-4 py-3 font-medium">
                        Why
                      </th>
                    </tr>
                  </thead>
                  <tbody>
                    {decisionRows.map(([situation, preference, reason]) => (
                      <tr key={situation} className="border-t align-top">
                        <th
                          scope="row"
                          className="px-4 py-3 font-medium text-[var(--color-ink)]"
                        >
                          {situation}
                        </th>
                        <td className="px-4 py-3 font-mono text-xs text-[var(--color-key)]">
                          {preference}
                        </td>
                        <td className="px-4 py-3 leading-6 text-[var(--color-muted)]">
                          {reason}
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </section>

            <section id="tradeoffs" className="mt-14 scroll-mt-24">
              <h2 className="text-3xl">
                The differences that matter in production
              </h2>

              <h3 className="mt-8 text-xl">Latency and backpressure</h3>
              <p className="mt-3 leading-7 text-[var(--color-muted)]">
                An NDJSON producer can hand off a record as soon as that record
                is complete. The downstream process can then slow the producer
                through the pipe or socket instead of accepting an entire
                collection first. Framing alone does not guarantee low latency,
                though. User-space and pipe buffers still exist, so live
                producers need an explicit flush policy.
              </p>

              <h3 className="mt-8 text-xl">Memory</h3>
              <p className="mt-3 leading-7 text-[var(--color-muted)]">
                NDJSON makes bounded-memory code natural: read one line, parse
                one value, do the work, discard it. A specialized streaming JSON
                parser can also walk a large array incrementally, so JSON does
                not inherently require loading the whole file. Many everyday
                APIs and libraries do load a JSON document in one call, which is
                where the practical difference appears.
              </p>

              <h3 className="mt-8 text-xl">Failure recovery</h3>
              <p className="mt-3 leading-7 text-[var(--color-muted)]">
                A malformed NDJSON line has an obvious boundary. A reader may
                stop, quarantine that line, or report it and continue. The
                format does not choose the policy for you. With one JSON
                document, a syntax error can make the whole document invalid,
                even when most records look intact.
              </p>

              <h3 className="mt-8 text-xl">
                Appending, concatenating, and splitting
              </h3>
              <p className="mt-3 leading-7 text-[var(--color-muted)]">
                Appending a complete line to a newline-terminated NDJSON file
                preserves its structure. Concatenating two correctly
                newline-terminated NDJSON files does too. Two JSON arrays cannot
                be concatenated into a valid single document, and appending an
                item means editing the surrounding array. Line-based tools can
                also split NDJSON at record boundaries, provided records never
                contain physical newlines.
              </p>

              <h3 className="mt-8 text-xl">Schema evolution</h3>
              <p className="mt-3 leading-7 text-[var(--color-muted)]">
                Neither format supplies a schema. If fields change over time,
                put a version in each record or version the stream contract
                outside the file. Per-record framing makes mixed versions
                possible, but it does not make them safe automatically.
              </p>
            </section>

            <section id="jc-rs" className="mt-14 scroll-mt-24">
              <h2 className="text-3xl">
                Produce NDJSON from command output with jc-rs
              </h2>
              <p className="mt-3 leading-7 text-[var(--color-muted)]">
                jc-rs streaming parsers have names ending in{" "}
                <code className="font-mono text-sm">-s</code>. They consume
                input line by line and, by default, write one compact JSON
                record per line. The{" "}
                <code className="font-mono text-sm">-u</code> option means
                “unbuffer”: it flushes stdout after every emitted record, which
                matters for a pipe that stays open.
              </p>

              <CodeSample label="Follow an access log and keep server errors">
                {`tail -f /var/log/nginx/access.log \\
  | jc-rs -u --clf-s \\
  | jq -c 'select(.status >= 500)'`}
              </CodeSample>

              <p className="mt-4 leading-7 text-[var(--color-muted)]">
                Each part has a separate job.{" "}
                <Link
                  href="/parsers/clf-s"
                  className="text-[var(--color-key)] underline-offset-4 hover:underline"
                >
                  <code className="font-mono text-sm">--clf-s</code>
                </Link>{" "}
                recognizes Common or Combined Log Format,{" "}
                <code className="font-mono text-sm">-u</code> makes records
                visible immediately, and{" "}
                <code className="font-mono text-sm">jq -c</code> filters while
                keeping every result on one line. Leaving off{" "}
                <code className="font-mono text-sm">-u</code> does not change
                the NDJSON shape; it only allows buffered writes.
              </p>

              <div className="mt-6 rounded-xl border-l-4 border-l-[var(--color-num)] bg-[var(--color-surface)] p-5">
                <p className="font-mono text-[11px] tracking-wide text-[var(--color-num)] uppercase">
                  Important distinction
                </p>
                <p className="mt-2 leading-7 text-[var(--color-muted)]">
                  <code className="font-mono text-sm">-u</code> does not turn a
                  batch parser into a streaming parser. Pick a streaming parser
                  such as{" "}
                  <Link
                    href="/parsers/syslog-s"
                    className="text-[var(--color-key)] underline-offset-4 hover:underline"
                  >
                    <code className="font-mono text-sm">--syslog-s</code>
                  </Link>
                  ,{" "}
                  <Link
                    href="/parsers/git-log-s"
                    className="text-[var(--color-key)] underline-offset-4 hover:underline"
                  >
                    <code className="font-mono text-sm">--git-log-s</code>
                  </Link>
                  , or <code className="font-mono text-sm">--clf-s</code>, then
                  add <code className="font-mono text-sm">-u</code> when the
                  consumer must see each record immediately.
                </p>
              </div>

              <h3 className="mt-9 text-xl">
                A finite file does not need per-record flushing
              </h3>
              <CodeSample label="Convert a finished access log">
                {`cat access.log | jc-rs --clf-s > access.ndjson`}
              </CodeSample>
              <p className="mt-4 leading-7 text-[var(--color-muted)]">
                The output remains one JSON object per line. Letting jc-rs
                buffer writes avoids a flush for every record and is the
                sensible default when the input will close.
              </p>

              <h3 className="mt-9 text-xl">
                Keep going after a bad streaming record
              </h3>
              <CodeSample label="Record both successes and parse errors">
                {`ls -l | jc-rs -qq --ls-s > checked.ndjson
jq -c 'select(._jc_meta.success == false)' checked.ndjson`}
              </CodeSample>
              <p className="mt-4 leading-7 text-[var(--color-muted)]">
                By default, a streaming parse error stops the run. With{" "}
                <code className="font-mono text-sm">-qq</code>, jc-rs continues
                and adds{" "}
                <code className="font-mono text-sm">_jc_meta.success</code> to
                emitted records. A failed line becomes its own error record with
                the original line and error text, so the gap is visible rather
                than silently dropped. This is parser-specific: parsers that
                deliberately preserve unknown input in an{" "}
                <code className="font-mono text-sm">unparsable</code> field
                still consider that record successfully handled. Check the{" "}
                <Link
                  href="/parsers/ls-s"
                  className="text-[var(--color-key)] underline-offset-4 hover:underline"
                >
                  streaming parser&apos;s schema
                </Link>{" "}
                before routing failures.
              </p>
            </section>

            <section id="convert" className="mt-14 scroll-mt-24">
              <h2 className="text-3xl">
                Convert JSON to JSONL, and JSONL back to JSON
              </h2>
              <p className="mt-3 leading-7 text-[var(--color-muted)]">
                If the source JSON is a top-level array,{" "}
                <code className="font-mono text-sm">jq</code> can write each
                element as one compact line:
              </p>
              <CodeSample label="JSON array → NDJSON / JSONL">
                {`jq -c '.[]' records.json > records.jsonl`}
              </CodeSample>
              <p className="mt-4 leading-7 text-[var(--color-muted)]">
                The reverse operation slurps every JSON value from the input
                stream into an array:
              </p>
              <CodeSample label="NDJSON / JSONL → JSON array">
                {`jq -s '.' records.jsonl > records.json`}
              </CodeSample>
              <p className="mt-4 leading-7 text-[var(--color-muted)]">
                <code className="font-mono text-sm">jq -s</code> holds the
                collected values in memory. That is fine for a bounded file that
                fits comfortably, but it removes the memory advantage of
                record-at-a-time processing. If the next system accepts NDJSON,
                keep the stream line-delimited instead of building an array only
                to split it again.
              </p>
            </section>

            <section id="pitfalls" className="mt-14 scroll-mt-24">
              <h2 className="text-3xl">
                Four easy ways to break a JSON Lines pipeline
              </h2>
              <ol className="mt-6 space-y-5 text-[var(--color-muted)]">
                <li className="pl-1 leading-7">
                  <strong className="font-medium text-[var(--color-ink)]">
                    Pretty-printing the stream.
                  </strong>{" "}
                  Multi-line indentation destroys the physical line boundary.
                  Keep the transport compact and pretty-print only the record
                  you are inspecting.
                </li>
                <li className="pl-1 leading-7">
                  <strong className="font-medium text-[var(--color-ink)]">
                    Writing logs to stdout beside JSON.
                  </strong>{" "}
                  A progress message becomes a malformed record. Send
                  diagnostics to stderr and reserve stdout for data.
                </li>
                <li className="pl-1 leading-7">
                  <strong className="font-medium text-[var(--color-ink)]">
                    Forgetting the final newline.
                  </strong>{" "}
                  Many readers accept the last line without one, but a trailing
                  newline makes safe concatenation and shell processing less
                  surprising.
                </li>
                <li className="pl-1 leading-7">
                  <strong className="font-medium text-[var(--color-ink)]">
                    Assuming framing is validation.
                  </strong>{" "}
                  One line can still contain invalid JSON or a valid value with
                  the wrong fields. Validate syntax and schema at the boundary
                  that owns the contract.
                </li>
              </ol>
            </section>

            <section id="faq" className="mt-14 scroll-mt-24">
              <h2 className="text-3xl">Common questions</h2>
              <div className="mt-6 divide-y rounded-xl border bg-[var(--color-surface)] px-5 sm:px-6">
                <Question title="Is an NDJSON file valid JSON?">
                  Each record is valid JSON. A file containing two or more
                  records is not one JSON document because it has several
                  top-level values without an enclosing array.
                </Question>
                <Question title="What is the difference between NDJSON and JSONL?">
                  Usually only the name and file extension. Both are commonly
                  used for one complete JSON value per line. Confirm the exact
                  contract when an API specifies a media type, encoding, or
                  final-newline rule.
                </Question>
                <Question title="Can JSON Lines contain arrays or primitive values?">
                  Yes. Every line may hold any valid JSON value. Objects are the
                  safest convention for record streams because fields can evolve
                  without relying on array position.
                </Question>
                <Question title="Should I use NDJSON for configuration files?">
                  Usually not. Configuration is normally one nested document, so
                  JSON, YAML, or TOML communicates that shape better. See the{" "}
                  <Link
                    href="/guides/json-vs-yaml-vs-toml"
                    className="text-[var(--color-key)] underline-offset-4 hover:underline"
                  >
                    JSON, YAML, TOML, and XML comparison
                  </Link>
                  .
                </Question>
              </div>
            </section>

            <section className="mt-14 rounded-xl border bg-[var(--color-surface)] p-6 sm:p-7">
              <p className="font-mono text-[11px] tracking-wide text-[var(--color-key)] uppercase">
                Choose the framing
              </p>
              <h2 className="mt-3 text-2xl">
                Match the format to the consumer
              </h2>
              <p className="mt-3 leading-7 text-[var(--color-muted)]">
                Send JSON when the consumer needs the collection as one value.
                Send NDJSON when it must handle each record as it arrives, and
                make the producer&apos;s flush behavior explicit. jc-rs supplies
                line-oriented parsers for logs and long-running command output
                while leaving downstream tools with ordinary JSON fields.
              </p>
              <div className="mt-5 flex flex-wrap gap-3">
                <Link
                  href="/install"
                  className="rounded-md bg-[var(--color-key)] px-4 py-2 text-sm font-medium text-[var(--color-surface)] transition-opacity hover:opacity-90"
                >
                  Install jc-rs
                </Link>
                <Link
                  href="/parsers"
                  className="rounded-md border px-4 py-2 text-sm transition-colors hover:border-[var(--color-key)]"
                >
                  Find a streaming parser
                </Link>
              </div>
            </section>
          </div>
        </div>
      </article>
    </div>
  );
}

function CodeSample({ label, children }: { label: string; children: string }) {
  return (
    <figure className="mt-5 min-w-0 overflow-hidden rounded-lg border bg-[var(--color-sunk)]">
      <figcaption className="border-b px-4 py-2 font-mono text-[10px] tracking-wide text-[var(--color-faint)] uppercase">
        {label}
      </figcaption>
      <pre className="overflow-x-auto p-4 font-mono text-xs leading-relaxed">
        <code>{children}</code>
      </pre>
    </figure>
  );
}

function Question({ title, children }: { title: string; children: ReactNode }) {
  return (
    <section className="py-5 first:pt-6 last:pb-6">
      <h3 className="text-lg">{title}</h3>
      <p className="mt-2 leading-7 text-[var(--color-muted)]">{children}</p>
    </section>
  );
}
