import type { Metadata } from "next";
import Link from "next/link";
import { site } from "@/lib/site";

const canonical = "/guides/git-log-to-json";
const title = "Git log to JSON without fragile escaping";
const description =
  "Convert git log to JSON without hand-built pretty-format objects. Parse Git output with jc-rs, then query authors, dates, messages, and file stats with jq.";
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
  about: ["git log to JSON", "Git history", "jq", "JSON escaping"],
};

export default function GitLogToJsonGuide() {
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
        <span>git log to JSON</span>
      </nav>

      <header className="mt-5 max-w-3xl">
        <p className="font-mono text-xs tracking-wide text-[var(--color-muted)] uppercase">
          Git workflow guide
        </p>
        <h1 className="mt-3 text-4xl sm:text-5xl">
          Git log to JSON without hand-rolled escaping
        </h1>
        <p className="mt-5 text-lg leading-relaxed text-[var(--color-muted)]">
          Git prints its normal history, jc-rs parses each commit into a JSON
          array, and jq shapes the report. Commit subjects, names, and
          multi-line messages are serialized as data instead of being spliced
          into a JSON template.
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
          Examples exercised against a real repository and the Git parser
          fixture suite
        </p>
      </header>

      <div
        aria-label="The Git history processing pipeline"
        className="mt-9 grid overflow-hidden rounded-xl border bg-[var(--color-surface)] sm:grid-cols-[1fr_auto_1fr_auto_1fr_auto_1fr]"
      >
        <Stage label="History" value="git log" />
        <Arrow />
        <Stage label="Parse" value="jc-rs --git-log" accent />
        <Arrow />
        <Stage label="Data" value="JSON array" />
        <Arrow />
        <Stage label="Report" value="jq" />
      </div>

      <nav
        aria-label="On this page"
        className="mt-9 rounded-xl border bg-[var(--color-surface)] p-5"
      >
        <p className="font-mono text-[11px] tracking-wide text-[var(--color-faint)] uppercase">
          On this page
        </p>
        <div className="mt-3 grid gap-x-8 gap-y-2 text-sm sm:grid-cols-2">
          <a href="#quick-start" className="hover:text-[var(--color-key)]">
            Convert the default log
          </a>
          <a href="#escaping" className="hover:text-[var(--color-key)]">
            Why pretty-format JSON breaks
          </a>
          <a href="#formats" className="hover:text-[var(--color-key)]">
            Default, fuller, oneline, and stat
          </a>
          <a href="#recipes" className="hover:text-[var(--color-key)]">
            Useful Git and jq recipes
          </a>
          <a href="#scripts" className="hover:text-[var(--color-key)]">
            Use it safely in scripts
          </a>
          <a href="#limits" className="hover:text-[var(--color-key)]">
            Boundaries and failure modes
          </a>
        </div>
      </nav>

      <div className="mt-14 max-w-3xl space-y-16">
        <section id="quick-start" className="scroll-mt-24">
          <h2 className="text-3xl">The direct conversion</h2>
          <p className="mt-4 leading-relaxed text-[var(--color-muted)]">
            Run this inside a repository.{" "}
            <code className="font-mono text-sm">--no-decorate</code> keeps
            branch and tag labels out of the commit hash field; jc-rs reads the
            standard commit blocks and writes one JSON object per commit.
          </p>
          <CodeBlock label="Bash">
            {`git log -n 20 --no-decorate |
  jc-rs --git-log > commits.json`}
          </CodeBlock>
          <p className="mt-5 leading-relaxed text-[var(--color-muted)]">
            A default record includes the full commit hash, author name and
            email, the displayed date, the complete commit message, and parsed
            epoch fields. Empty author names and email addresses remain null
            instead of disappearing.
          </p>
          <div className="mt-6 overflow-hidden rounded-xl border bg-[var(--color-surface)]">
            <FieldRow
              name="commit"
              meaning="Full commit hash from the commit header"
              type="string"
            />
            <FieldRow
              name="author / author_email"
              meaning="Identity from the Author line"
              type="string | null"
            />
            <FieldRow
              name="date"
              meaning="Git's displayed author date"
              type="string"
            />
            <FieldRow
              name="message"
              meaning="Subject and body, with line breaks preserved"
              type="string"
            />
            <FieldRow
              name="epoch / epoch_utc"
              meaning="Parsed timestamp values when available"
              type="number | null"
            />
          </div>
          <p className="mt-4 text-sm text-[var(--color-faint)]">
            You can also use jc-rs magic syntax:{" "}
            <code className="font-mono">jc-rs git log -n 20 --no-decorate</code>
            . The explicit pipe is easier to extend with Git options and makes
            the data boundary obvious in a script.
          </p>
        </section>

        <section id="escaping" className="scroll-mt-24">
          <h2 className="text-3xl">
            Why JSON-shaped pretty formats are brittle
          </h2>
          <p className="mt-4 leading-relaxed text-[var(--color-muted)]">
            A common shortcut asks Git to print text that resembles a JSON
            object:
          </p>
          <CodeBlock label="Fragile: output only looks like JSON">
            {`git log --format='{"commit":"%H","subject":"%s"}'`}
          </CodeBlock>
          <p className="mt-5 leading-relaxed text-[var(--color-muted)]">
            Git substitutes the subject directly into that template. A subject
            containing a double quote, backslash, tab, or control character
            needs JSON escaping that the format string does not provide.
            Multi-line bodies add another delimiter problem, and joining the
            objects into an array introduces comma handling.
          </p>
          <CodeBlock label="Structured: serialization happens after parsing">
            {`git log --no-decorate |
  jc-rs --git-log |
  jq 'map({commit, author, date, message})'`}
          </CodeBlock>
          <div className="mt-5 rounded-xl border-l-2 border-[var(--color-key)] bg-[var(--color-surface)] px-5 py-4">
            <p className="text-sm leading-relaxed text-[var(--color-muted)]">
              jc-rs creates valid JSON from Git&apos;s record grammar. jq
              filters that JSON. Neither commit text nor a shell variable is
              evaluated as jq or JSON source code.
            </p>
          </div>
        </section>

        <section id="formats" className="scroll-mt-24">
          <h2 className="text-3xl">Use a Git format the parser understands</h2>
          <p className="mt-4 leading-relaxed text-[var(--color-muted)]">
            The parser covers Git&apos;s familiar log styles and stat blocks.
            Choose the least verbose style that contains the fields you need.
          </p>

          <div className="mt-6 overflow-hidden rounded-xl border bg-[var(--color-surface)]">
            <FormatRow
              command="git log --no-decorate"
              adds="Commit, author, date, and message"
              note="Best default"
            />
            <FormatRow
              command="git log --format=fuller --no-decorate"
              adds="Committer identity and commit date"
              note="Adds commit_by fields"
            />
            <FormatRow
              command="git log --stat --no-decorate"
              adds="Changed files, insertions, and deletions"
              note="Adds nested stats"
            />
            <FormatRow
              command="git log --oneline --no-abbrev-commit"
              adds="Full hash and subject only"
              note="Full 40-character hash is required"
            />
          </div>

          <CodeBlock label="Committer as well as author">
            {`git log -n 10 --format=fuller --no-decorate |
  jc-rs --git-log |
  jq 'map({commit, author, commit_by, date, commit_by_date})'`}
          </CodeBlock>
          <p className="mt-4 text-sm text-[var(--color-faint)]">
            Avoid arbitrary custom <code className="font-mono">--format</code>{" "}
            strings here. The parser needs recognizable commit, identity, date,
            message, and stat lines; it cannot infer the meaning of an unrelated
            delimiter scheme.
          </p>
        </section>

        <section id="recipes" className="scroll-mt-24">
          <h2 className="text-3xl">Practical Git log to JSON recipes</h2>

          <h3 className="mt-7 text-xl">A compact release-note feed</h3>
          <CodeBlock label="Bash">
            {`git log v1.4.0..HEAD --no-merges --no-decorate |
  jc-rs --git-log |
  jq 'map({
    commit: .commit[0:12],
    author,
    subject: (.message | split("\\n")[0])
  })'`}
          </CodeBlock>

          <h3 className="mt-8 text-xl">Commit counts by author</h3>
          <CodeBlock label="Bash">
            {`git log --since='2026-01-01' --no-decorate |
  jc-rs --git-log |
  jq 'group_by(.author)
    | map({author: .[0].author, commits: length})
    | sort_by(-.commits)'`}
          </CodeBlock>

          <h3 className="mt-8 text-xl">Changed-line totals from stat output</h3>
          <CodeBlock label="Bash">
            {`git log -n 50 --stat --no-decorate |
  jc-rs --git-log |
  jq 'map({
    commit: .commit[0:12],
    files: (.stats.files_changed // 0),
    lines: ((.stats.insertions // 0) + (.stats.deletions // 0))
  })'`}
          </CodeBlock>

          <h3 className="mt-8 text-xl">Only commits touching one path</h3>
          <CodeBlock label="Bash">
            {`git log --no-decorate -- website/src/ |
  jc-rs --git-log |
  jq 'map({commit, author, message})'`}
          </CodeBlock>
          <p className="mt-4 text-sm text-[var(--color-faint)]">
            Let Git perform revision, merge, author, date, and path selection.
            It can walk its own graph more efficiently than jq can filter an
            unnecessarily large history afterward.
          </p>
        </section>

        <section id="scripts" className="scroll-mt-24">
          <h2 className="text-3xl">
            Treat an empty history and a failed command separately
          </h2>
          <p className="mt-4 leading-relaxed text-[var(--color-muted)]">
            An empty revision range is valid and becomes an empty JSON array. A
            bad revision or a failed parser is an error. Bash needs{" "}
            <code className="font-mono text-sm">pipefail</code> to preserve
            failures from the left side of the pipeline; jq can enforce a
            non-empty result when the job requires one.
          </p>
          <CodeBlock label="Bash script">
            {`#!/usr/bin/env bash
set -Eeuo pipefail

range=\${1:-HEAD~10..HEAD}

if ! git log "$range" --no-decorate |
  jc-rs --git-log |
  jq -e '
    if length > 0
    then map({commit, author, message})
    else error("revision range contains no commits")
    end
  ' >commits.json
then
  printf 'could not build commit report for %s\\n' "$range" >&2
  exit 1
fi`}
          </CodeBlock>
          <p className="mt-4 text-sm text-[var(--color-faint)]">
            The range is passed to Git as one quoted argument. It is never
            interpolated into the jq program.
          </p>
        </section>

        <section id="limits" className="scroll-mt-24">
          <h2 className="text-3xl">Boundaries worth knowing</h2>
          <ul className="mt-5 space-y-4 text-[var(--color-muted)]">
            <li>
              <strong className="text-[var(--color-ink)]">
                This parses log text, not the Git object database.
              </strong>{" "}
              Git remains responsible for revision walking, path filters,
              mailmap behavior, and date selection.
            </li>
            <li>
              <strong className="text-[var(--color-ink)]">
                Disable decorations for stable hashes.
              </strong>{" "}
              Branch and tag labels are presentation text; keep them out of the
              commit field unless they are intentionally part of your report.
            </li>
            <li>
              <strong className="text-[var(--color-ink)]">
                Oneline input needs full hashes.
              </strong>{" "}
              Add <code className="font-mono text-sm">--no-abbrev-commit</code>.
              The abbreviated form is not enough for the parser&apos;s oneline
              record boundary.
            </li>
            <li>
              <strong className="text-[var(--color-ink)]">
                Large histories are batch output.
              </strong>{" "}
              The regular Git parser returns one array after input ends. Limit
              the range in Git rather than converting years of history for a
              ten-commit report.
            </li>
          </ul>
        </section>

        <section className="rounded-xl border bg-[var(--color-surface)] p-6">
          <h2 className="text-2xl">
            Check one real record before reusing the filter
          </h2>
          <p className="mt-3 text-sm leading-relaxed text-[var(--color-muted)]">
            The parser page shows a Git fixture beside its exact JSON output.
            Confirm the fields you plan to query; the Bash guide covers the
            failure handling used in the script above.
          </p>
          <div className="mt-5 grid gap-3 sm:grid-cols-2">
            <RelatedLink
              href="/parsers/git-log"
              title="git_log parser reference"
            >
              Exact CLI argument, fixture pair, schema, and compatibility
              evidence.
            </RelatedLink>
            <RelatedLink
              href="/guides/bash-jc-rs-jq"
              title="Bash, jc-rs and jq"
            >
              Quoting, pipefail, stderr, jq exit status, and empty input.
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

function FieldRow({
  name,
  meaning,
  type,
}: {
  name: string;
  meaning: string;
  type: string;
}) {
  return (
    <div className="grid gap-2 border-b px-5 py-4 last:border-b-0 sm:grid-cols-[10rem_1fr_7rem]">
      <code className="font-mono text-xs text-[var(--color-key)]">{name}</code>
      <p className="text-sm text-[var(--color-muted)]">{meaning}</p>
      <code className="font-mono text-xs text-[var(--color-faint)]">
        {type}
      </code>
    </div>
  );
}

function FormatRow({
  command,
  adds,
  note,
}: {
  command: string;
  adds: string;
  note: string;
}) {
  return (
    <div className="grid gap-2 border-b px-5 py-4 last:border-b-0 sm:grid-cols-[minmax(0,1.2fr)_1fr_0.8fr]">
      <code className="overflow-x-auto font-mono text-xs text-[var(--color-key)]">
        {command}
      </code>
      <p className="text-sm">{adds}</p>
      <p className="text-sm text-[var(--color-muted)]">{note}</p>
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
