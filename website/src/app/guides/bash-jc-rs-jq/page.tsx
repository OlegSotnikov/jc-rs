import type { Metadata } from "next";
import Link from "next/link";
import { site } from "@/lib/site";

const canonical = "/guides/bash-jc-rs-jq";
const title = "Bash, jc-rs and jq: reliable command-output pipelines";
const description =
  "Use jq safely in Bash when commands print human-readable output. jc-rs creates JSON; jq filters it. Covers quoting, pipefail, stderr, empty input, and streams.";
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
  twitter: { card: "summary_large_image", title, description, images: [site.socialImage.url] },
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
  about: ["bash jq", "jq bash", "command output to JSON", "shell pipelines"],
};

export default function BashJcRsJqGuide() {
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
        <span>Bash + jq</span>
      </nav>

      <header className="mt-5 max-w-3xl">
        <p className="font-mono text-xs tracking-wide text-[var(--color-muted)] uppercase">
          Shell pipeline guide
        </p>
        <h1 className="mt-3 text-4xl sm:text-5xl">
          Use jc-rs and jq safely in Bash
        </h1>
        <p className="mt-5 text-lg leading-relaxed text-[var(--color-muted)]">
          jq reads JSON; it does not parse the columns printed by{" "}
          <code className="font-mono text-base">ps</code>,{" "}
          <code className="font-mono text-base">df</code>, or{" "}
          <code className="font-mono text-base">ss</code>. When the command has
          no suitable JSON mode, jc-rs establishes the schema before jq runs. A
          production Bash script also needs to preserve quoting, keep stderr out
          of the data, and notice failures from every stage.
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
          Commands checked with Bash, jq 1.7, and the jc-rs CLI
        </p>
      </header>

      <div
        aria-label="The command-output query pipeline"
        className="mt-9 grid overflow-hidden rounded-xl border bg-[var(--color-surface)] sm:grid-cols-[1fr_auto_1fr_auto_1fr_auto_1fr]"
      >
        <Stage label="Producer" value="command stdout" />
        <Arrow />
        <Stage label="Parser" value="jc-rs" accent />
        <Arrow />
        <Stage label="Query" value="jq" />
        <Arrow />
        <Stage label="Consumer" value="JSON / text" />
      </div>

      <nav
        aria-label="On this page"
        className="mt-9 rounded-xl border bg-[var(--color-surface)] p-5"
      >
        <p className="font-mono text-[11px] tracking-wide text-[var(--color-faint)] uppercase">
          On this page
        </p>
        <div className="mt-3 grid gap-x-8 gap-y-2 text-sm sm:grid-cols-2">
          <a href="#roles" className="hover:text-[var(--color-key)]">
            Know which tool does what
          </a>
          <a href="#jq-shapes" className="hover:text-[var(--color-key)]">
            Keep or unwrap the JSON array
          </a>
          <a href="#quoting" className="hover:text-[var(--color-key)]">
            Quote jq programs and variables
          </a>
          <a href="#pipefail" className="hover:text-[var(--color-key)]">
            Preserve pipeline failures
          </a>
          <a href="#stderr" className="hover:text-[var(--color-key)]">
            Keep stderr out of the data
          </a>
          <a href="#empty" className="hover:text-[var(--color-key)]">
            Define empty-input behavior
          </a>
        </div>
      </nav>

      <div className="mt-14 max-w-3xl space-y-16">
        <section id="roles" className="scroll-mt-24">
          <h2 className="text-3xl">jc-rs creates JSON; jq queries it</h2>
          <p className="mt-4 leading-relaxed text-[var(--color-muted)]">
            A reliable Bash and jq pipeline has three contracts. The command
            writes the format its parser expects. jc-rs converts that format to
            a documented JSON schema. jq selects, reshapes, or renders values
            from that schema.
          </p>
          <CodeBlock label="Bash">
            {`LC_ALL=C ps aux |
  jc-rs --ps |
  jq 'map(select(.mem_percent >= 5))'`}
          </CodeBlock>
          <div className="mt-5 overflow-hidden rounded-xl border bg-[var(--color-surface)]">
            <RoleRow
              tool="ps aux"
              owns="Process collection"
              doesNotOwn="JSON quoting or field names"
            />
            <RoleRow
              tool="jc-rs --ps"
              owns="Columns, types, nulls, and JSON serialization"
              doesNotOwn="Which processes you want"
            />
            <RoleRow
              tool="jq"
              owns="Filtering and output shape"
              doesNotOwn="The human-readable ps grammar"
            />
          </div>
          <p className="mt-4 text-sm text-[var(--color-faint)]">
            If the producer has a stable native JSON flag, prefer it and omit
            jc-rs. The extra parser is useful only when human-readable output is
            the available interface.
          </p>
        </section>

        <section id="jq-shapes" className="scroll-mt-24">
          <h2 className="text-3xl">
            Decide whether the next stage needs an array
          </h2>
          <p className="mt-4 leading-relaxed text-[var(--color-muted)]">
            Most non-streaming command parsers return an array of records. jq
            can preserve that array for another JSON consumer, or unwrap it into
            a stream for text processing.
          </p>
          <div className="mt-6 overflow-hidden rounded-xl border bg-[var(--color-surface)]">
            <JqRow
              expression="map(select(.use_percent >= 80))"
              shape="one JSON array"
              use="API payload, file, or another JSON stage"
            />
            <JqRow
              expression=".[] | select(.use_percent >= 80)"
              shape="successive JSON objects"
              use="streaming into another jq-aware command"
            />
            <JqRow
              expression="-r '.[] | .mounted_on'"
              shape="raw text lines"
              use="human output or a line-oriented shell command"
            />
            <JqRow
              expression="-c '.[]'"
              shape="compact JSON, one value per line"
              use="NDJSON output"
            />
          </div>
          <CodeBlock label="Keep the matching rows as JSON">
            {`LC_ALL=C df -h |
  jc-rs --df |
  jq 'map(select(.use_percent >= 80))'`}
          </CodeBlock>
          <CodeBlock label="Render two fields as tab-separated text">
            {`LC_ALL=C df -h |
  jc-rs --df |
  jq -r '.[]
    | select(.use_percent >= 80)
    | [.mounted_on, ((.use_percent | tostring) + "%")]
    | @tsv'`}
          </CodeBlock>
          <p className="mt-4 text-sm text-[var(--color-faint)]">
            Use <code className="font-mono">-r</code> only at the boundary where
            JSON becomes text. Keeping JSON longer avoids a second round of
            shell splitting and escaping.
          </p>
        </section>

        <section id="quoting" className="scroll-mt-24">
          <h2 className="text-3xl">
            Single-quote the jq program; pass data with arguments
          </h2>
          <p className="mt-4 leading-relaxed text-[var(--color-muted)]">
            In Bash, a jq program normally belongs in single quotes. That
            prevents the shell from expanding{" "}
            <code className="font-mono text-sm">$variables</code>, backslashes,
            and wildcard characters before jq sees them. Values from Bash should
            cross the boundary with{" "}
            <code className="font-mono text-sm">--arg</code> or{" "}
            <code className="font-mono text-sm">--argjson</code>, never by
            building jq source code.
          </p>
          <h3 className="mt-7 text-xl">String input</h3>
          <CodeBlock label="Bash">
            {`wanted_user=alice

LC_ALL=C ps aux |
  jc-rs --ps |
  jq --arg user "$wanted_user" 'map(select(.user == $user))'`}
          </CodeBlock>
          <h3 className="mt-8 text-xl">Numeric input</h3>
          <CodeBlock label="Bash">
            {`limit=\${LIMIT:-80}
case $limit in
  (''|*[!0-9]*) printf 'LIMIT must be an integer\\n' >&2; exit 2 ;;
esac

LC_ALL=C df -h |
  jc-rs --df |
  jq --argjson limit "$limit" 'map(select(.use_percent >= $limit))'`}
          </CodeBlock>
          <div className="mt-5 rounded-xl border-l-2 border-[var(--color-num)] bg-[var(--color-surface)] px-5 py-4">
            <p className="font-mono text-xs text-[var(--color-num)]">Avoid</p>
            <p className="mt-2 font-mono text-xs">
              Building a double-quoted jq program that interpolates
              $wanted_user.
            </p>
            <p className="mt-3 text-sm leading-relaxed text-[var(--color-muted)]">
              Quotes or jq syntax inside the value can break the program or
              change its meaning.
              <code className="ml-1 font-mono">--arg</code> serializes it as
              data instead.
            </p>
          </div>
        </section>

        <section id="pipefail" className="scroll-mt-24">
          <h2 className="text-3xl">Make an upstream failure fail the script</h2>
          <p className="mt-4 leading-relaxed text-[var(--color-muted)]">
            Bash normally reports the status of the last command in a pipeline.
            If the producer or jc-rs fails but jq exits successfully, the script
            can appear healthy. Enable{" "}
            <code className="font-mono text-sm">pipefail</code> and use
            jq&apos;s <code className="font-mono text-sm">-e</code> mode when
            the jq result itself is a condition.
          </p>
          <CodeBlock label="Fail if any stage fails">
            {`#!/usr/bin/env bash
set -Eeuo pipefail

if ! {
  LC_ALL=C df -h |
    jc-rs --df |
    jq -e '
      if type == "array"
      then map(select(.use_percent >= 90))
      else error("expected an array from the df parser")
      end
    '
} >full-filesystems.json 2>pipeline.err; then
  printf 'disk report failed; see pipeline.err\\n' >&2
  exit 1
fi`}
          </CodeBlock>
          <p className="mt-4 text-sm leading-relaxed text-[var(--color-faint)]">
            <code className="font-mono">set -e</code> alone is not a substitute
            for <code className="font-mono">pipefail</code>. An empty JSON array
            is also a valid, truthy jq value; test its length separately if “no
            records” is an error for your job.
          </p>
        </section>

        <section id="stderr" className="scroll-mt-24">
          <h2 className="text-3xl">
            Keep stderr separate from structured data
          </h2>
          <p className="mt-4 leading-relaxed text-[var(--color-muted)]">
            Pipes carry stdout. That is exactly what you want: diagnostics
            remain on stderr while parseable data moves right. Redirect the
            whole pipeline&apos;s stderr to a separate file when a scheduled job
            needs a record of failures.
          </p>
          <div className="mt-6 grid gap-4 sm:grid-cols-2">
            <RuleCard title="Good">
              <code className="font-mono text-xs">
                {"{ command | jc-rs --parser | jq ...; } 2>pipeline.err"}
              </code>
              <p className="mt-3 text-sm text-[var(--color-muted)]">
                The parser receives only the command&apos;s stdout.
              </p>
            </RuleCard>
            <RuleCard title="Bad" warning>
              <code className="font-mono text-xs">
                command 2&gt;&amp;1 | jc-rs --parser
              </code>
              <p className="mt-3 text-sm text-[var(--color-muted)]">
                An error message is mixed into the data and may become a bogus
                row.
              </p>
            </RuleCard>
          </div>
          <p className="mt-5 leading-relaxed text-[var(--color-muted)]">
            Locale is part of the input contract too. For commands whose
            headings or numbers are localized, set{" "}
            <code className="font-mono text-sm">LC_ALL=C</code> on the producer,
            as the examples above do. It does not need to be exported for the
            whole script.
          </p>
        </section>

        <section id="empty" className="scroll-mt-24">
          <h2 className="text-3xl">
            Define what empty input means for this job
          </h2>
          <p className="mt-4 leading-relaxed text-[var(--color-muted)]">
            Parsers do not all treat empty input identically: for some formats
            an empty collection is meaningful; for others it is malformed input.
            After a successful parse, decide whether zero records is acceptable
            rather than relying on implicit jq behavior.
          </p>
          <CodeBlock label="Require at least one matching process">
            {`if ! LC_ALL=C ps aux |
  jc-rs --ps |
  jq -e --arg user "$wanted_user" '
    map(select(.user == $user))
    | if length > 0 then . else error("no matching process") end
  ' >processes.json
then
  printf 'process lookup failed or returned no rows\\n' >&2
  exit 1
fi`}
          </CodeBlock>
          <p className="mt-4 leading-relaxed text-[var(--color-muted)]">
            For one scalar destined for a shell variable, use raw output and
            require a value. Keep the expansion quoted afterward:
          </p>
          <CodeBlock label="Capture one PID">
            {`pid=$(LC_ALL=C ps aux |
  jc-rs --ps |
  jq -er --arg user "$wanted_user" '
    first(.[] | select(.user == $user)) | .pid
  ')

printf 'first PID: %s\\n' "$pid"`}
          </CodeBlock>
        </section>

        <section>
          <h2 className="text-3xl">The same rules apply to a live stream</h2>
          <p className="mt-4 leading-relaxed text-[var(--color-muted)]">
            Streaming parsers produce NDJSON: each record is a complete JSON
            value. jq naturally reads that sequence. Use compact output to
            preserve the one-record-per-line contract.
          </p>
          <CodeBlock label="Bash">
            {`tail -F /var/log/nginx/access.log |
  jc-rs -u --clf-s |
  jq -c 'select(.status >= 500)'`}
          </CodeBlock>
          <p className="mt-4 text-sm text-[var(--color-faint)]">
            Because this pipeline is intentionally long-lived, its final status
            is available only when it exits. Run production monitors under a
            supervisor that restarts failed processes and captures stderr.
          </p>
        </section>

        <section className="rounded-xl border bg-[var(--color-surface)] p-6">
          <h2 className="text-2xl">
            Check the parser schema before writing jq
          </h2>
          <p className="mt-3 text-sm leading-relaxed text-[var(--color-muted)]">
            The parser pages show fixture-derived field names and output shapes.
            Check one before committing a jq expression to a script, especially
            when the source command varies by operating system.
          </p>
          <div className="mt-5 grid gap-3 sm:grid-cols-2">
            <RelatedLink href="/parsers/ps" title="ps parser">
              Process fields such as pid, user, cpu_percent and command.
            </RelatedLink>
            <RelatedLink href="/parsers/df" title="df parser">
              Filesystems, mount points, sizes and utilization.
            </RelatedLink>
            <RelatedLink
              href="/guides/logs-to-json"
              title="Convert logs to JSON"
            >
              Batch arrays and live NDJSON from CLF, syslog, and CEF.
            </RelatedLink>
            <RelatedLink href="/guides/git-log-to-json" title="Git log to JSON">
              Query commit history without hand-rolled JSON escaping.
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

function RoleRow({
  tool,
  owns,
  doesNotOwn,
}: {
  tool: string;
  owns: string;
  doesNotOwn: string;
}) {
  return (
    <div className="grid gap-2 border-b px-5 py-4 last:border-b-0 sm:grid-cols-[8rem_1fr_1fr]">
      <code className="font-mono text-sm text-[var(--color-key)]">{tool}</code>
      <p className="text-sm">{owns}</p>
      <p className="text-sm text-[var(--color-muted)]">Not: {doesNotOwn}</p>
    </div>
  );
}

function JqRow({
  expression,
  shape,
  use,
}: {
  expression: string;
  shape: string;
  use: string;
}) {
  return (
    <div className="grid gap-2 border-b px-5 py-4 last:border-b-0 sm:grid-cols-[minmax(0,1.35fr)_0.8fr_1fr]">
      <code className="overflow-x-auto font-mono text-xs text-[var(--color-key)]">
        {expression}
      </code>
      <p className="text-sm">{shape}</p>
      <p className="text-sm text-[var(--color-muted)]">{use}</p>
    </div>
  );
}

function RuleCard({
  title,
  warning = false,
  children,
}: {
  title: string;
  warning?: boolean;
  children: React.ReactNode;
}) {
  return (
    <div className="rounded-xl border bg-[var(--color-surface)] p-5">
      <p
        className={`font-mono text-xs ${warning ? "text-[var(--color-num)]" : "text-[var(--color-str)]"}`}
      >
        {title}
      </p>
      <div className="mt-3">{children}</div>
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
