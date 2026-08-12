import type { Metadata } from "next";
import Link from "next/link";
import type { ReactNode } from "react";
import { site } from "@/lib/site";

const canonical = "/guides/parsing-command-output-reliably";
const title = "How to parse command output reliably";
const description =
  "Build reliable command-output parsers by controlling locale, whitespace, line wrapping, versions, stderr, exit status, and the JSON schema your script consumes.";
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
  url: `${site.origin}${canonical}`,
  mainEntityOfPage: `${site.origin}${canonical}`,
  datePublished: published,
  dateModified: published,
  articleSection: "Command-line guides",
  author: { "@type": "Person", name: site.author, url: site.authorUrl },
  publisher: { "@type": "Organization", name: site.name, url: site.origin },
  about: [
    "parse command output",
    "command output parser",
    "LC_ALL",
    "stderr",
    "schema contracts",
  ],
};

const failureModes = [
  [
    "Locale",
    "Headings, decimal marks, dates, and diagnostics change",
    "Set and test the producer locale",
  ],
  [
    "Whitespace",
    "Padding is mistaken for a delimiter",
    "Parse the command grammar, not split() output",
  ],
  [
    "Width",
    "Fields wrap, truncate, or become ellipses",
    "Disable paging and request full-width output",
  ],
  [
    "Version",
    "Columns or meanings drift",
    "Pin invocations and run fixtures across supported versions",
  ],
  [
    "stderr",
    "Diagnostics become data",
    "Keep stdout and stderr separate; preserve both statuses",
  ],
  [
    "Schema",
    "Valid JSON changes underneath the consumer",
    "Assert fields, types, nullability, and record shape",
  ],
] as const;

export default function ParsingCommandOutputReliablyGuide() {
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
        <span>reliable command-output parsing</span>
      </nav>

      <header className="mt-5 max-w-4xl">
        <p className="font-mono text-xs tracking-wide text-[var(--color-muted)] uppercase">
          Engineering guide
        </p>
        <h1 className="mt-3 max-w-3xl text-4xl sm:text-5xl">
          Parse command output reliably
        </h1>
        <p className="mt-5 max-w-3xl text-lg leading-relaxed text-[var(--color-muted)]">
          Reliable command-output parsing starts with a precise input contract:
          one command, one option set, a controlled locale and width, known
          record boundaries, separate error handling, and a tested output
          schema. Whether the implementation uses awk, Rust, or Python matters
          less than holding those inputs constant.
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
          · Published and checked{" "}
          <time dateTime={published}>August 11, 2026</time> · Examples exercised
          with jc-rs parser fixtures and shell failure cases
        </p>
      </header>

      <div className="mt-9 overflow-x-auto rounded-xl border bg-[var(--color-surface)]">
        <table className="w-full min-w-[46rem] border-collapse text-left text-sm">
          <thead className="bg-[var(--color-sunk)] font-mono text-[11px] tracking-wide text-[var(--color-faint)] uppercase">
            <tr>
              <th scope="col" className="px-4 py-3 font-medium">
                Boundary
              </th>
              <th scope="col" className="px-4 py-3 font-medium">
                Typical break
              </th>
              <th scope="col" className="px-4 py-3 font-medium">
                Control
              </th>
            </tr>
          </thead>
          <tbody>
            {failureModes.map(([boundary, failure, control]) => (
              <tr key={boundary} className="border-t align-top">
                <th
                  scope="row"
                  className="px-4 py-3 font-medium text-[var(--color-ink)]"
                >
                  {boundary}
                </th>
                <td className="px-4 py-3 leading-6 text-[var(--color-muted)]">
                  {failure}
                </td>
                <td className="px-4 py-3 leading-6 text-[var(--color-muted)]">
                  {control}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      <nav
        aria-label="On this page"
        className="mt-9 rounded-xl border bg-[var(--color-surface)] p-5"
      >
        <p className="font-mono text-[11px] tracking-wide text-[var(--color-faint)] uppercase">
          On this page
        </p>
        <div className="mt-3 grid gap-x-8 gap-y-2 text-sm sm:grid-cols-2">
          <a href="#contract" className="hover:text-[var(--color-key)]">
            Start with an input contract
          </a>
          <a href="#locale" className="hover:text-[var(--color-key)]">
            Locale is input
          </a>
          <a href="#whitespace" className="hover:text-[var(--color-key)]">
            Whitespace and delimiters
          </a>
          <a href="#wrapping" className="hover:text-[var(--color-key)]">
            Wrapping and terminal width
          </a>
          <a href="#versions" className="hover:text-[var(--color-key)]">
            Version drift
          </a>
          <a href="#stderr" className="hover:text-[var(--color-key)]">
            stderr and exit status
          </a>
          <a href="#schema" className="hover:text-[var(--color-key)]">
            Schema contracts
          </a>
          <a href="#tests" className="hover:text-[var(--color-key)]">
            A useful test matrix
          </a>
        </div>
      </nav>

      <div className="mt-14 max-w-3xl space-y-16">
        <section id="contract" className="scroll-mt-24">
          <h2 className="text-3xl">
            Start with the producer, not a regular expression
          </h2>
          <p className="mt-4 leading-relaxed text-[var(--color-muted)]">
            Write down the exact invocation before writing parsing code. Options
            often select a different grammar:{" "}
            <code className="font-mono text-sm">ps -ef</code> and{" "}
            <code className="font-mono text-sm">ps aux</code> do not merely show
            more or fewer rows; they select different columns. A parser tested
            against one cannot silently claim the other.
          </p>
          <p className="mt-4 leading-relaxed text-[var(--color-muted)]">
            First look for a structured or purpose-built interface. Use
            documented native JSON when it exists. For systemd, for example,{" "}
            <code className="font-mono text-sm">systemctl status</code> is
            explicitly human-facing, while{" "}
            <code className="font-mono text-sm">systemctl show</code> exposes
            normalized properties for programs. Only parse display text when it
            is the artifact or interface you genuinely have.
          </p>
          <div className="mt-6 grid gap-4 sm:grid-cols-2">
            <ContractCard label="Producer contract">
              Command, options, platform, version, locale, width, and whether
              stdout is attached to a terminal.
            </ContractCard>
            <ContractCard label="Consumer contract">
              Top-level JSON shape, field names, value types, nullability,
              ordering assumptions, and failure policy.
            </ContractCard>
          </div>
          <p className="mt-4 text-sm leading-relaxed text-[var(--color-faint)]">
            The{" "}
            <Link
              href="/guides/native-json-or-jc-rs"
              className="text-[var(--color-key)] underline-offset-4 hover:underline"
            >
              native JSON or jc-rs decision guide
            </Link>{" "}
            applies that rule to ip, lsblk, journalctl, and text-only fallbacks.
          </p>
        </section>

        <section id="locale" className="scroll-mt-24">
          <h2 className="text-3xl">Locale is part of the bytes you parse</h2>
          <p className="mt-4 leading-relaxed text-[var(--color-muted)]">
            Locale can translate headings and month names, change collation, and
            choose decimal or thousands separators. A parser that recognizes{" "}
            <code className="font-mono text-sm">Filesystem</code> cannot infer
            that an unfamiliar translated word means the same field. Set the
            locale on the producer so it emits the grammar represented by your
            fixtures.
          </p>
          <CodeBlock label="Constrain only this producer">
            {`LC_ALL=C df -P |
  jc-rs --df |
  jq 'map({filesystem, mounted_on, use_percent})'`}
          </CodeBlock>
          <p className="mt-4 leading-relaxed text-[var(--color-muted)]">
            <code className="font-mono text-sm">LC_ALL=C</code> has higher
            precedence than the other locale categories for that process.
            Applying it to the left side of the pipe is deliberate: df is the
            program rendering localized text. You do not need to export a
            process-wide locale for unrelated commands.
          </p>
          <div className="mt-5 rounded-xl border-l-2 border-[var(--color-num)] bg-[var(--color-surface)] px-5 py-4">
            <p className="text-sm leading-relaxed text-[var(--color-muted)]">
              C locale is not a universal repair flag. It cannot make a GNU
              parser understand a different operating system&apos;s columns or
              make the wrong jc-rs parser fit the input. It removes one
              controlled source of variation; platform and invocation still
              matter.
            </p>
          </div>
        </section>

        <section id="whitespace" className="scroll-mt-24">
          <h2 className="text-3xl">
            Decide whether whitespace is syntax or presentation
          </h2>
          <p className="mt-4 leading-relaxed text-[var(--color-muted)]">
            Aligned tables use spaces for at least two jobs: separating columns
            and padding short values. A free-text final column may contain the
            same spaces. Empty cells, tabs, and a long identifier can shift
            everything to the right. A blanket{" "}
            <code className="font-mono text-sm">split_whitespace()</code>{" "}
            therefore has no way to reconstruct the intended row unless the
            command&apos;s grammar supplies more information.
          </p>
          <p className="mt-4 leading-relaxed text-[var(--color-muted)]">
            awk or sed is a sound choice when the producer promises an
            unambiguous delimiter and constrains the field contents. A short
            parser is then easier to audit than a large one. Git ref names
            cannot contain a tab, so this invocation creates a real
            tab-delimited contract before jq turns each line into an object:
          </p>
          <CodeBlock label="An explicit delimiter makes simple parsing reasonable">
            {`git for-each-ref \\
  --format='%(refname)%09%(objectname)' \\
  refs/heads/ |
  jq -Rn '[
    inputs
    | split("\\t")
    | {ref: .[0], object: .[1]}
  ]'`}
          </CodeBlock>
          <p className="mt-4 leading-relaxed text-[var(--color-muted)]">
            By contrast, the command column in ps and the description column in
            systemctl are not safely recovered by splitting every run of spaces.
            A matching parser uses the known header and row rules for that
            command. Inspect the fixture-backed schemas on the{" "}
            <Link
              href="/parsers/ps"
              className="text-[var(--color-key)] underline-offset-4 hover:underline"
            >
              ps parser
            </Link>{" "}
            and{" "}
            <Link
              href="/parsers/systemctl"
              className="text-[var(--color-key)] underline-offset-4 hover:underline"
            >
              systemctl parser
            </Link>{" "}
            pages before choosing fields downstream.
          </p>
        </section>

        <section id="wrapping" className="scroll-mt-24">
          <h2 className="text-3xl">Make non-interactive output explicit</h2>
          <p className="mt-4 leading-relaxed text-[var(--color-muted)]">
            Many commands inspect whether stdout is a terminal. That decision
            can enable a pager, color, a tree, shortened headings, ellipses, or
            width-based wrapping. A cron job and an interactive shell may
            therefore receive different bytes from the same-looking command.
          </p>
          <CodeBlock label="Processes without width truncation">
            {`LC_ALL=C ps auxww |
  jc-rs --ps > processes.json`}
          </CodeBlock>
          <CodeBlock label="systemctl table without pager, color, or ellipses">
            {`LC_ALL=C SYSTEMD_COLORS=0 \\
  systemctl --all --no-pager --full --plain |
  jc-rs --systemctl > units.json`}
          </CodeBlock>
          <p className="mt-4 text-sm leading-relaxed text-[var(--color-faint)]">
            Use the producer&apos;s own options instead of hoping that a large{" "}
            <code className="font-mono">COLUMNS</code> value will suppress every
            display feature. Then test both piped and pseudo-terminal execution
            if your application ever runs the command under a terminal
            allocator.
          </p>
        </section>

        <section id="versions" className="scroll-mt-24">
          <h2 className="text-3xl">
            Treat version drift as a schema migration
          </h2>
          <p className="mt-4 leading-relaxed text-[var(--color-muted)]">
            Human output changes for good reasons: a new column, clearer units,
            a renamed state, extra summary lines, or different rendering of
            missing values. The parser may still return valid JSON while
            assigning the wrong meaning to one value, which is more dangerous
            than a clean parse failure.
          </p>
          <div className="mt-6 overflow-hidden rounded-xl border bg-[var(--color-surface)]">
            <VersionRow
              change="Column added in the middle"
              risk="Every positional field after it shifts"
            />
            <VersionRow
              change="Heading renamed"
              risk="Header-derived JSON keys change or detection fails"
            />
            <VersionRow
              change="Dash changes from missing to literal"
              risk="Null becomes a string, or vice versa"
            />
            <VersionRow
              change="Unit convention changes"
              risk="A correct-looking number has a different scale"
            />
            <VersionRow
              change="Footer or warning added"
              risk="A diagnostic is mistaken for another record"
            />
          </div>
          <p className="mt-5 leading-relaxed text-[var(--color-muted)]">
            Record the supported producer versions in test metadata, but assert
            behavior rather than accepting a version string alone. A
            distribution may backport output changes without adopting the
            upstream release number you expected. jc-rs keeps fixtures from
            several operating systems for precisely this reason.
          </p>
        </section>

        <section id="stderr" className="scroll-mt-24">
          <h2 className="text-3xl">
            stderr is evidence, not another input column
          </h2>
          <p className="mt-4 leading-relaxed text-[var(--color-muted)]">
            A normal pipe carries stdout and leaves diagnostics on stderr.
            Preserve that split. Redirecting{" "}
            <code className="font-mono text-sm">2&gt;&amp;1</code> before the
            parser can turn a permission warning, transient network error, or
            usage message into a plausible but false record.
          </p>
          <div className="mt-6 grid gap-4 sm:grid-cols-2">
            <RuleCard label="Data path" value="command | jc-rs --parser" good>
              Only the producer&apos;s stdout reaches the parser.
            </RuleCard>
            <RuleCard
              label="Contaminated path"
              value="command 2>&1 | jc-rs --parser"
            >
              Diagnostics and records become indistinguishable bytes.
            </RuleCard>
          </div>
          <p className="mt-5 leading-relaxed text-[var(--color-muted)]">
            Exit status is a separate channel too. Bash normally reports the
            final command&apos;s status for a pipeline, so enable{" "}
            <code className="font-mono text-sm">pipefail</code> if an upstream
            failure must fail the job.
          </p>
          <CodeBlock label="A compact pipeline with failure propagation">
            {`#!/usr/bin/env bash
set -Eeuo pipefail

LC_ALL=C df -P |
  jc-rs --df |
  jq -e 'select(type == "array")' > filesystems.json`}
          </CodeBlock>
          <p className="mt-4 text-sm leading-relaxed text-[var(--color-faint)]">
            curl verbose traces are an intentional exception because curl writes
            that trace to stderr. Capture it as a named artifact, check
            curl&apos;s status, and only then parse it; the exact pattern is in
            the{" "}
            <Link
              href="/guides/curl-headers-to-json"
              className="text-[var(--color-key)] underline-offset-4 hover:underline"
            >
              curl headers to JSON guide
            </Link>
            .
          </p>
        </section>

        <section id="schema" className="scroll-mt-24">
          <h2 className="text-3xl">
            Validate the JSON contract, not just JSON syntax
          </h2>
          <p className="mt-4 leading-relaxed text-[var(--color-muted)]">
            Successful JSON parsing proves that brackets and strings are well
            formed. It does not prove that the top level is an array, that a
            utilization field is numeric, that null is allowed, or that the
            command itself succeeded. Assert the smallest schema the next stage
            relies on.
          </p>
          <CodeBlock label="Capture each stage, then enforce the df schema">
            {`#!/usr/bin/env bash
set -Eeuo pipefail

work_dir=$(mktemp -d)
trap 'rm -rf -- "$work_dir"' EXIT

if ! LC_ALL=C df -P >"$work_dir/df.out" 2>"$work_dir/df.err"; then
  cat "$work_dir/df.err" >&2
  exit 1
fi

if ! jc-rs --df \
  <"$work_dir/df.out" \
  >"$work_dir/df.json" \
  2>"$work_dir/jc-rs.err"; then
  cat "$work_dir/jc-rs.err" >&2
  exit 1
fi

jq -e '
  select(
    type == "array" and
    length > 0 and
    all(.[];
      (.filesystem | type == "string") and
      (.mounted_on | type == "string") and
      (.use_percent | type == "number")
    )
  )
' <"$work_dir/df.json" > filesystems.json`}
          </CodeBlock>
          <p className="mt-4 leading-relaxed text-[var(--color-muted)]">
            This longer form is useful in a scheduled job because it identifies
            the stage that failed and retains each diagnostic until the script
            exits. For ordinary interactive work, the shorter{" "}
            <code className="font-mono text-sm">pipefail</code> pipeline is
            often enough. Choose based on the observability the job needs.
          </p>
        </section>

        <section id="tests" className="scroll-mt-24">
          <h2 className="text-3xl">
            Test the assumptions, not just the happy path
          </h2>
          <p className="mt-4 leading-relaxed text-[var(--color-muted)]">
            One golden sample verifies a happy path. A production parser needs
            samples selected for the assumptions most likely to be false.
          </p>
          <div className="mt-6 grid gap-3 sm:grid-cols-2">
            <TestCard title="Platforms and versions">
              Oldest and newest supported producer on every supported OS family.
            </TestCard>
            <TestCard title="Locale">
              C plus at least one locale with translated text or different
              numeric formatting.
            </TestCard>
            <TestCard title="Width and values">
              Very long names, spaces, empty values, Unicode, and non-terminal
              execution.
            </TestCard>
            <TestCard title="Cardinality">
              Zero rows, one row, many rows, and repeated sections or headers.
            </TestCard>
            <TestCard title="Failure paths">
              Permission denied, missing file, partial output, and non-zero
              producer status.
            </TestCard>
            <TestCard title="Consumer schema">
              Types, nullability, required fields, top-level shape, and semantic
              invariants.
            </TestCard>
          </div>
          <p className="mt-5 leading-relaxed text-[var(--color-muted)]">
            Keep the raw input next to the expected JSON. When a producer
            changes, the diff then shows whether you are extending a documented
            grammar or accidentally teaching the parser to accept one
            machine&apos;s corrupted output.
          </p>
        </section>

        <section
          aria-labelledby="sources"
          className="rounded-xl border bg-[var(--color-surface)] p-6"
        >
          <h2 id="sources" className="text-2xl">
            Primary sources and fixture-backed parsers
          </h2>
          <p className="mt-3 text-sm leading-relaxed text-[var(--color-muted)]">
            The locale behavior follows the{" "}
            <a
              href="https://pubs.opengroup.org/onlinepubs/9699919799/basedefs/V1_chap08.html"
              className="text-[var(--color-key)] underline-offset-4 hover:underline"
            >
              POSIX environment-variable specification
            </a>
            . The distinction between human-facing status and program-facing
            properties is documented by{" "}
            <a
              href="https://www.freedesktop.org/software/systemd/man/latest/systemctl.html"
              className="text-[var(--color-key)] underline-offset-4 hover:underline"
            >
              systemctl
            </a>
            . jc-rs keeps its input/output evidence in the{" "}
            <a
              href={`${site.repo}/tree/master/tests/fixtures`}
              className="text-[var(--color-key)] underline-offset-4 hover:underline"
            >
              fixture suite
            </a>
            .
          </p>
          <div className="mt-5 grid gap-3 sm:grid-cols-2">
            <RelatedLink href="/parsers/df" title="df parser">
              Filesystem fields and fixture-derived JSON.
            </RelatedLink>
            <RelatedLink href="/parsers/ps" title="ps parser">
              Process schemas for supported ps layouts.
            </RelatedLink>
            <RelatedLink href="/parsers/ss" title="ss parser">
              Socket output with command-specific parsing.
            </RelatedLink>
            <RelatedLink
              href="/guides/bash-jc-rs-jq"
              title="Bash, jc-rs, and jq"
            >
              Quoting, pipefail, empty input, and jq output shapes.
            </RelatedLink>
          </div>
        </section>
      </div>
    </article>
  );
}

function ContractCard({
  label,
  children,
}: {
  label: string;
  children: ReactNode;
}) {
  return (
    <div className="rounded-xl border bg-[var(--color-surface)] p-5">
      <p className="font-mono text-xs text-[var(--color-key)]">{label}</p>
      <p className="mt-3 text-sm leading-relaxed text-[var(--color-muted)]">
        {children}
      </p>
    </div>
  );
}

function VersionRow({ change, risk }: { change: string; risk: string }) {
  return (
    <div className="grid gap-2 border-b px-5 py-4 last:border-b-0 sm:grid-cols-[12rem_1fr]">
      <p className="text-sm font-medium text-[var(--color-ink)]">{change}</p>
      <p className="text-sm leading-relaxed text-[var(--color-muted)]">
        {risk}
      </p>
    </div>
  );
}

function RuleCard({
  label,
  value,
  good = false,
  children,
}: {
  label: string;
  value: string;
  good?: boolean;
  children: ReactNode;
}) {
  return (
    <div className="rounded-xl border bg-[var(--color-surface)] p-5">
      <p
        className={`font-mono text-xs ${good ? "text-[var(--color-str)]" : "text-[var(--color-num)]"}`}
      >
        {label}
      </p>
      <code className="mt-3 block overflow-x-auto font-mono text-xs">
        {value}
      </code>
      <p className="mt-3 text-sm leading-relaxed text-[var(--color-muted)]">
        {children}
      </p>
    </div>
  );
}

function TestCard({ title, children }: { title: string; children: ReactNode }) {
  return (
    <div className="rounded-lg border bg-[var(--color-surface)] p-4">
      <p className="text-sm font-medium text-[var(--color-ink)]">{title}</p>
      <p className="mt-1.5 text-xs leading-relaxed text-[var(--color-muted)]">
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
  children: ReactNode;
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
  children: ReactNode;
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
