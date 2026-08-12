import type { Metadata } from "next";
import Link from "next/link";
import { site } from "@/lib/site";

const canonical = "/guides/logs-to-json";
const title = "Convert logs to JSON: syslog, CLF and CEF";
const description =
  "Convert syslog, Apache or Nginx access logs, and CEF events to JSON with jc-rs. Covers parser selection, finished files, and live NDJSON streams.";
const published = "2026-08-11";

export const metadata: Metadata = {
  title,
  description,
  alternates: { canonical },
  openGraph: {
    type: "article",
    title,
    description,
    url: `${site.origin}${canonical}`,
    publishedTime: published,
    modifiedTime: published,
    authors: [site.authorUrl],
    images: [site.socialImage],
  },
  twitter: {
    card: "summary_large_image",
    title,
    description,
    images: [site.socialImage.url],
  },
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
  about: ["log to JSON", "syslog", "Common Log Format", "Common Event Format"],
};

const clfLine =
  '203.0.113.7 - - [11/Aug/2026:14:22:09 +0000] "GET /health HTTP/1.1" 503 19 "-" "curl/8.7.1"';

export default function LogsToJsonGuide() {
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
        <span>logs to JSON</span>
      </nav>

      <header className="mt-5 max-w-3xl">
        <p className="font-mono text-xs tracking-wide text-[var(--color-muted)] uppercase">
          Log parsing guide
        </p>
        <h1 className="mt-3 text-4xl sm:text-5xl">
          How to convert logs to JSON without guessing the schema
        </h1>
        <p className="mt-5 text-lg leading-relaxed text-[var(--color-muted)]">
          A <code className="font-mono text-base">.log</code> extension says
          nothing about the records inside. Identify the grammar first, then use
          the matching parser: syslog, Common or Combined Log Format, or CEF.
          jc-rs turns each record into structured JSON; jq can then select the
          fields and events you need.
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
          · Published <time dateTime={published}>August 11, 2026</time> ·
          Examples checked against the jc-rs parser fixtures
        </p>
      </header>

      <div
        aria-label="The log processing pipeline"
        className="mt-9 grid overflow-hidden rounded-xl border bg-[var(--color-surface)] sm:grid-cols-[1fr_auto_1fr_auto_1fr_auto_1fr]"
      >
        <Stage label="Input" value="log record" />
        <Arrow />
        <Stage label="Structure" value="jc-rs" accent />
        <Arrow />
        <Stage label="Data" value="JSON / NDJSON" />
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
            Choose the parser by format
          </a>
          <a href="#access-log" className="hover:text-[var(--color-key)]">
            Worked access-log example
          </a>
          <a href="#file-or-stream" className="hover:text-[var(--color-key)]">
            File versus live stream
          </a>
          <a href="#native-json" className="hover:text-[var(--color-key)]">
            Prefer native JSON when available
          </a>
          <a href="#validation" className="hover:text-[var(--color-key)]">
            Validate before trusting the result
          </a>
          <a href="#recipes" className="hover:text-[var(--color-key)]">
            Useful jq recipes
          </a>
        </div>
      </nav>

      <div className="mt-14 max-w-3xl space-y-16">
        <section id="choose" className="scroll-mt-24">
          <h2 className="text-3xl">Choose the log parser by format</h2>
          <p className="mt-4 leading-relaxed text-[var(--color-muted)]">
            Before converting a log file to JSON, inspect a few complete
            records. The producer and its configured output format matter; the
            filename does not. Syslog, CLF, and CEF are separate grammars with
            different fields and escaping rules.
          </p>

          <div className="mt-6 overflow-hidden rounded-xl border bg-[var(--color-surface)]">
            <ParserRow
              source="Apache or Nginx access log"
              grammar="Common / Combined Log Format"
              batch="--clf"
              stream="--clf-s"
              href="/parsers/clf"
            />
            <ParserRow
              source="System or network log"
              grammar="RFC 5424 syslog, with BSD syslog fallback"
              batch="--syslog"
              stream="--syslog-s"
              href="/parsers/syslog"
            />
            <ParserRow
              source="Security appliance or SIEM export"
              grammar="Common Event Format (CEF)"
              batch="--cef"
              stream="--cef-s"
              href="/parsers/cef"
            />
            <div className="grid gap-2 px-5 py-4 sm:grid-cols-[11rem_1fr]">
              <p className="text-sm font-medium">Application-specific lines</p>
              <p className="text-sm text-[var(--color-muted)]">
                Use the application&apos;s native JSON mode or define a schema
                for that exact format. A general log parser cannot reliably
                infer fields from arbitrary prose.
              </p>
            </div>
          </div>

          <p className="mt-4 text-sm text-[var(--color-faint)]">
            The streaming variants have their own parser names ending in{" "}
            <code className="font-mono">-s</code>. They emit one JSON value per
            input record. The <code className="font-mono">-u</code> option tells
            jc-rs to flush each value as soon as it is ready.
          </p>
        </section>

        <section id="access-log" className="scroll-mt-24">
          <h2 className="text-3xl">A complete access-log conversion</h2>
          <p className="mt-4 leading-relaxed text-[var(--color-muted)]">
            This Combined Log Format record contains an address, timestamp, HTTP
            request, status, byte count, referrer, and user agent. Splitting on
            spaces would break as soon as a quoted value contains a space.
          </p>
          <CodeBlock label="Input record">{clfLine}</CodeBlock>
          <p className="mt-5 leading-relaxed text-[var(--color-muted)]">
            jc-rs parses the full record. jq is used only after that conversion
            to make the example output shorter:
          </p>
          <CodeBlock label="Bash">
            {`printf '%s\\n' '${clfLine}' |
  jc-rs --clf |
  jq '.[0] | {host, request_method, request_url, status, bytes}'`}
          </CodeBlock>
          <CodeBlock label="Result">
            {`{
  "host": "203.0.113.7",
  "request_method": "GET",
  "request_url": "/health",
  "status": 503,
  "bytes": 19
}`}
          </CodeBlock>
          <div className="mt-5 rounded-xl border-l-2 border-[var(--color-key)] bg-[var(--color-surface)] px-5 py-4">
            <p className="text-sm leading-relaxed text-[var(--color-muted)]">
              Keep the roles separate:{" "}
              <strong className="text-[var(--color-ink)]">
                jc-rs creates JSON
              </strong>{" "}
              from the log grammar, and{" "}
              <strong className="text-[var(--color-ink)]">
                jq filters JSON
              </strong>
              . jq does not know how quoted CLF fields, syslog structured data,
              or CEF extensions are encoded.
            </p>
          </div>
        </section>

        <section id="file-or-stream" className="scroll-mt-24">
          <h2 className="text-3xl">
            Convert a finished file or follow a live one
          </h2>
          <h3 className="mt-7 text-xl">Finished file: write one JSON array</h3>
          <p className="mt-3 leading-relaxed text-[var(--color-muted)]">
            Use the regular parser when the input has an end. The result is one
            valid JSON array, convenient for archiving or passing to a program
            that expects a complete document.
          </p>
          <CodeBlock label="Bash">
            {`jc-rs --clf < /var/log/nginx/access.log > access.json
jc-rs --syslog < exported-syslog.log > syslog.json
jc-rs --cef < security-events.cef > security-events.json`}
          </CodeBlock>

          <h3 className="mt-9 text-xl">
            Growing file: emit NDJSON immediately
          </h3>
          <p className="mt-3 leading-relaxed text-[var(--color-muted)]">
            A file followed by{" "}
            <code className="font-mono text-sm">tail -F</code> may never reach
            EOF. Use a streaming parser so each complete line becomes a JSON
            object while the writer stays open. jq accepts successive JSON
            values and, with <code className="font-mono text-sm">-c</code>,
            writes one compact result per line.
          </p>
          <CodeBlock label="Bash">
            {`tail -F /var/log/nginx/access.log |
  jc-rs -u --clf-s |
  jq -c 'select(.status >= 500)'`}
          </CodeBlock>
          <p className="mt-4 text-sm text-[var(--color-faint)]">
            This output is NDJSON, not a JSON array. Keep it line-oriented for
            another streaming consumer, or collect it later with{" "}
            <code className="font-mono">jq -s &apos;.&apos;</code>. See the{" "}
            <Link
              href="/guides/ndjson-vs-json-vs-jsonl"
              className="text-[var(--color-key)] underline-offset-4 hover:underline"
            >
              NDJSON, JSONL and JSON guide
            </Link>{" "}
            for the memory and recovery trade-offs.
          </p>
        </section>

        <section id="native-json" className="scroll-mt-24">
          <h2 className="text-3xl">
            If the producer already emits JSON, keep it
          </h2>
          <p className="mt-4 leading-relaxed text-[var(--color-muted)]">
            Parsing is for existing human-readable logs and systems whose output
            you cannot change. When you control the application, configure
            structured logging at the source. When a command already has a
            stable JSON mode, send that JSON straight to jq.
          </p>
          <CodeBlock label="Native JSON; jc-rs is not needed">
            {`journalctl -o json --since today |
  jq -c 'select(.PRIORITY == "3")'`}
          </CodeBlock>
          <p className="mt-4 leading-relaxed text-[var(--color-muted)]">
            Do not run already-valid JSON through{" "}
            <code className="font-mono text-sm">--syslog</code>, and do not
            serialize JSON inside a message string if the logger can emit fields
            natively. Source-side structure preserves types and avoids a parsing
            step altogether.
          </p>
        </section>

        <section id="validation" className="scroll-mt-24">
          <h2 className="text-3xl">
            Validate a sample before converting the archive
          </h2>
          <p className="mt-4 leading-relaxed text-[var(--color-muted)]">
            Successful parsing guarantees valid JSON serialization, not a
            correct interpretation of the source. Check that the input matched
            the expected grammar and that the fields mean what your downstream
            job assumes.
          </p>
          <ol className="mt-5 space-y-5 text-[var(--color-muted)]">
            <li className="grid grid-cols-[2rem_1fr] gap-3">
              <span className="font-mono text-sm text-[var(--color-key)]">
                01
              </span>
              <span>
                Run ten representative records, including missing fields,
                unusual user agents, and error lines. Compare the parsed values
                with the originals.
              </span>
            </li>
            <li className="grid grid-cols-[2rem_1fr] gap-3">
              <span className="font-mono text-sm text-[var(--color-key)]">
                02
              </span>
              <span>
                Count records carrying an{" "}
                <code className="font-mono text-sm">unparsable</code> field. The
                CLF and syslog parsers preserve lines they cannot classify
                instead of quietly inventing fields.
              </span>
            </li>
            <li className="grid grid-cols-[2rem_1fr] gap-3">
              <span className="font-mono text-sm text-[var(--color-key)]">
                03
              </span>
              <span>
                Keep diagnostics on stderr. Never merge stderr into the log
                stream with{" "}
                <code className="font-mono text-sm">2&gt;&amp;1</code>; an error
                message is not a log record in the selected grammar.
              </span>
            </li>
            <li className="grid grid-cols-[2rem_1fr] gap-3">
              <span className="font-mono text-sm text-[var(--color-key)]">
                04
              </span>
              <span>
                Inspect timestamps and nulls explicitly. Not every source
                carries a UTC offset or every optional CLF field, and a missing
                value should stay missing.
              </span>
            </li>
          </ol>
          <CodeBlock label="Check a completed conversion">
            {`jq '{
  records: length,
  unparsable: [.[] | select(has("unparsable"))] | length
}' access.json`}
          </CodeBlock>
          <p className="mt-4 text-sm text-[var(--color-faint)]">
            Log files often contain credentials, session identifiers, and
            customer data. A local CLI conversion keeps the file on the machine;
            still apply the same access controls to the JSON output as to the
            original log.
          </p>
        </section>

        <section id="recipes" className="scroll-mt-24">
          <h2 className="text-3xl">Queries worth keeping</h2>
          <h3 className="mt-7 text-xl">Count HTTP statuses</h3>
          <CodeBlock label="jq">
            {`jq 'group_by(.status)
  | map({status: .[0].status, requests: length})
  | sort_by(-.requests)' access.json`}
          </CodeBlock>

          <h3 className="mt-8 text-xl">Find high-severity CEF events</h3>
          <CodeBlock label="Bash">
            {`jc-rs --cef < security-events.cef |
  jq '[.[] | select((.agentSeverityNum // 0) >= 7)]'`}
          </CodeBlock>

          <h3 className="mt-8 text-xl">
            Watch emergency through error syslog severities
          </h3>
          <CodeBlock label="Bash">
            {`tail -F exported-syslog.log |
  jc-rs -u --syslog-s |
  jq -c 'select(.priority != null and (.priority % 8) <= 3)'`}
          </CodeBlock>
          <p className="mt-4 text-sm text-[var(--color-faint)]">
            The syslog PRI value combines facility and severity. Taking it
            modulo 8 recovers the severity number; 0 through 3 mean emergency,
            alert, critical, and error.
          </p>
        </section>

        <section className="rounded-xl border bg-[var(--color-surface)] p-6">
          <h2 className="text-2xl">Inspect the schema before writing jq</h2>
          <p className="mt-3 text-sm leading-relaxed text-[var(--color-muted)]">
            Each parser reference shows fixture-derived output. Use those field
            names in jq, and keep log parsing in jc-rs rather than rebuilding
            the source grammar in a filter.
          </p>
          <div className="mt-5 grid gap-3 sm:grid-cols-2">
            <RelatedLink href="/install" title="Install jc-rs">
              Prebuilt binaries, Cargo, Homebrew and Docker.
            </RelatedLink>
            <RelatedLink
              href="/guides/bash-jc-rs-jq"
              title="Use jc-rs with jq in Bash"
            >
              Quoting, pipefail, stderr and empty-input checks.
            </RelatedLink>
            <RelatedLink href="/parsers/clf" title="CLF parser reference">
              Command, schema example and fixture coverage.
            </RelatedLink>
            <RelatedLink href="/parsers/cef" title="CEF parser reference">
              CEF header, extension fields and JSON output.
            </RelatedLink>
          </div>
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

function ParserRow({
  source,
  grammar,
  batch,
  stream,
  href,
}: {
  source: string;
  grammar: string;
  batch: string;
  stream: string;
  href: string;
}) {
  return (
    <div className="grid gap-3 border-b px-5 py-4 last:border-b-0 sm:grid-cols-[11rem_1fr_auto] sm:items-center">
      <p className="text-sm font-medium">{source}</p>
      <p className="text-sm text-[var(--color-muted)]">{grammar}</p>
      <div className="flex items-center gap-3">
        <code className="font-mono text-xs text-[var(--color-key)]">
          {batch}
        </code>
        <Link
          href={`${href}-s`}
          className="font-mono text-xs hover:text-[var(--color-key)]"
        >
          {stream}
        </Link>
        <Link
          href={href}
          aria-label={`${source} parser reference`}
          className="text-sm hover:text-[var(--color-key)]"
        >
          ↗
        </Link>
      </div>
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
