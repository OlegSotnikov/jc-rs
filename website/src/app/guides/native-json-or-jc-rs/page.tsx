import type { Metadata } from "next";
import Link from "next/link";
import type { ReactNode } from "react";
import { site } from "@/lib/site";

const canonical = "/guides/native-json-or-jc-rs";
const title = "Native JSON flags or jc-rs? A command-by-command guide";
const description =
  "Choose between native JSON output and jc-rs for Linux command output. Practical guidance for ip -j, lsblk -J, journalctl -o json, and text-only tools.";
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
    "native JSON output",
    "ip -j",
    "lsblk -J",
    "journalctl JSON output",
    "command output to JSON",
  ],
};

export default function NativeJsonOrJcRsGuide() {
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
        <span>native JSON or jc-rs</span>
      </nav>

      <header className="mt-5 max-w-4xl">
        <p className="font-mono text-xs tracking-wide text-[var(--color-muted)] uppercase">
          Decision guide
        </p>
        <h1 className="mt-3 max-w-3xl text-4xl sm:text-5xl">
          Native JSON flags or jc-rs?
        </h1>
        <p className="mt-5 max-w-3xl text-lg leading-relaxed text-[var(--color-muted)]">
          Prefer a command&apos;s documented, stable JSON mode when your
          supported versions have one. On Linux that means starting with{" "}
          <code className="font-mono text-base">ip -j</code>,{" "}
          <code className="font-mono text-base">lsblk -J</code>, and{" "}
          <code className="font-mono text-base">journalctl -o json</code>. Use
          jc-rs when formatted text is the real interface you must consume, or
          when old and mixed systems cannot offer the native mode.
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
          <time dateTime={published}>August 11, 2026</time>
        </p>
      </header>

      <div className="mt-9 overflow-hidden rounded-xl border bg-[var(--color-surface)]">
        <div className="grid sm:grid-cols-[1fr_auto_1fr_auto_1fr] sm:items-stretch">
          <Stage label="Question" value="Documented JSON mode?" />
          <Arrow />
          <Stage label="Yes" value="Use native JSON" accent />
          <Arrow />
          <Stage label="No" value="Use a matching jc-rs parser" />
        </div>
        <p className="border-t px-5 py-3 text-sm text-[var(--color-muted)]">
          In both branches, pin the invocation and test the fields. JSON
          guarantees syntax, not a permanent application schema.
        </p>
      </div>

      <nav
        aria-label="On this page"
        className="mt-9 rounded-xl border bg-[var(--color-surface)] p-5"
      >
        <p className="font-mono text-[11px] tracking-wide text-[var(--color-faint)] uppercase">
          On this page
        </p>
        <div className="mt-3 grid gap-x-8 gap-y-2 text-sm sm:grid-cols-2">
          <a href="#rule" className="hover:text-[var(--color-key)]">
            The decision rule
          </a>
          <a href="#ip" className="hover:text-[var(--color-key)]">
            ip -j
          </a>
          <a href="#lsblk" className="hover:text-[var(--color-key)]">
            lsblk -J
          </a>
          <a href="#journalctl" className="hover:text-[var(--color-key)]">
            journalctl -o json
          </a>
          <a href="#jc-rs" className="hover:text-[var(--color-key)]">
            Where jc-rs belongs
          </a>
          <a href="#contract" className="hover:text-[var(--color-key)]">
            Make the schema explicit
          </a>
        </div>
      </nav>

      <div className="mt-14 max-w-3xl space-y-16">
        <section id="rule" className="scroll-mt-24">
          <h2 className="text-3xl">
            Use the closest structured interface to the source
          </h2>
          <p className="mt-4 leading-relaxed text-[var(--color-muted)]">
            Native JSON is produced before a command lays data out for a
            terminal. It does not depend on translated column headings, padding,
            tree-drawing characters, or the current screen width. Skipping that
            presentation layer removes an entire class of parsing failures and
            one process from the pipeline.
          </p>
          <p className="mt-4 leading-relaxed text-[var(--color-muted)]">
            The word <em>native</em> is not enough by itself. Prefer a mode
            documented by the command, available across the versions you
            support, and exercised by a schema test. Experimental JSON, an
            undocumented switch, or fields that change between installed
            releases still need compatibility work.
          </p>

          <div className="mt-6 overflow-hidden rounded-xl border bg-[var(--color-surface)]">
            <DecisionRow
              rank="1"
              choice="Documented native JSON"
              when="The producer exposes the records and fields you need."
            />
            <DecisionRow
              rank="2"
              choice="Documented machine format"
              when="JSON is absent, but the tool offers stable pairs, null delimiters, or another formal mode."
            />
            <DecisionRow
              rank="3"
              choice="jc-rs parser"
              when="Human-readable output is unavoidable and a parser exists for that exact command shape."
            />
            <DecisionRow
              rank="4"
              choice="Purpose-built adapter"
              when="You own a narrow text contract that no existing parser covers."
            />
          </div>
        </section>

        <section id="ip" className="scroll-mt-24">
          <p className="font-mono text-xs tracking-wide text-[var(--color-key)] uppercase">
            Native wins
          </p>
          <h2 className="mt-2 text-3xl">Use ip -j for addresses and routes</h2>
          <p className="mt-4 leading-relaxed text-[var(--color-muted)]">
            The iproute2 <code className="font-mono text-sm">-j</code> option
            asks the producer for JSON;{" "}
            <code className="font-mono text-sm">-p</code> only pretty-prints
            that JSON for a person. Keep compact output in pipelines and let jq
            choose the fields.
          </p>
          <CodeBlock label="Interface addresses as a smaller JSON report">
            {`ip -j address show |
  jq 'map({
    ifname,
    mtu,
    addresses: [.addr_info[]? | {family, local, prefixlen}]
  })'`}
          </CodeBlock>
          <CodeBlock label="Default routes">
            {`ip -j route show default |
  jq 'map({gateway, dev, metric: (.metric // 0)})'`}
          </CodeBlock>
          <p className="mt-4 text-sm leading-relaxed text-[var(--color-faint)]">
            Do not pipe <code className="font-mono">ip -j</code> into jc-rs. It
            is already JSON. For an older host where text from{" "}
            <code className="font-mono">ip route</code> is the only available
            artifact, use the{" "}
            <Link
              href="/parsers/ip-route"
              className="text-[var(--color-key)] underline-offset-4 hover:underline"
            >
              ip route parser
            </Link>{" "}
            against that text instead.
          </p>
        </section>

        <section id="lsblk" className="scroll-mt-24">
          <p className="font-mono text-xs tracking-wide text-[var(--color-key)] uppercase">
            Native wins
          </p>
          <h2 className="mt-2 text-3xl">Use lsblk -J, and name every column</h2>
          <p className="mt-4 leading-relaxed text-[var(--color-muted)]">
            <code className="font-mono text-sm">lsblk -J</code> returns a JSON
            object with a{" "}
            <code className="font-mono text-sm">blockdevices</code> array. The
            util-linux manual explicitly warns that default output can change,
            including defaults selected by convenience options. For a script,
            JSON plus an implicit column list is only half a contract.
          </p>
          <CodeBlock label="Stable lsblk invocation">
            {`lsblk --json --tree \\
  --output NAME,TYPE,SIZE,MOUNTPOINTS |
  jq '.blockdevices'`}
          </CodeBlock>
          <p className="mt-4 leading-relaxed text-[var(--color-muted)]">
            Request <code className="font-mono text-sm">--tree</code> when
            hierarchy matters, and keep{" "}
            <code className="font-mono text-sm">NAME</code> in the selected
            columns. Otherwise the presence of nested{" "}
            <code className="font-mono text-sm">children</code> is not a safe
            assumption. If you must process saved table output from a host
            without JSON support, the{" "}
            <Link
              href="/parsers/lsblk"
              className="text-[var(--color-key)] underline-offset-4 hover:underline"
            >
              jc-rs lsblk parser
            </Link>{" "}
            converts the familiar columns and adds typed fields such as byte
            counts.
          </p>
        </section>

        <section id="journalctl" className="scroll-mt-24">
          <p className="font-mono text-xs tracking-wide text-[var(--color-key)] uppercase">
            Native wins
          </p>
          <h2 className="mt-2 text-3xl">
            journalctl -o json is already a record stream
          </h2>
          <p className="mt-4 leading-relaxed text-[var(--color-muted)]">
            Journal entries are structured before they are rendered as short log
            lines. The <code className="font-mono text-sm">json</code> output
            mode writes one JSON object per line, so it is NDJSON rather than
            one surrounding array. jq accepts the sequence directly.
          </p>
          <CodeBlock label="Critical SSH service entries from the last hour">
            {`journalctl --unit=sshd.service \\
  --since='1 hour ago' \\
  --no-pager \\
  --output=json \\
  --output-fields=MESSAGE,PRIORITY,_SYSTEMD_UNIT |
  jq -c 'select((.PRIORITY | tonumber) <= 3)'`}
          </CodeBlock>
          <p className="mt-4 leading-relaxed text-[var(--color-muted)]">
            Keep it line-delimited for a streaming consumer. To build one
            bounded array, use{" "}
            <code className="font-mono text-sm">jq -s &apos;.&apos;</code> after
            applying a sensible time or row limit. The distinction is covered in
            the{" "}
            <Link
              href="/guides/ndjson-vs-json-vs-jsonl"
              className="text-[var(--color-key)] underline-offset-4 hover:underline"
            >
              NDJSON, JSONL, and JSON guide
            </Link>
            .
          </p>
          <div className="mt-5 rounded-xl border-l-2 border-[var(--color-key)] bg-[var(--color-surface)] px-5 py-4">
            <p className="text-sm leading-relaxed text-[var(--color-muted)]">
              Do not render the journal as{" "}
              <code className="font-mono">short</code> output and parse it back
              into fields. Use the journal&apos;s own JSON while you still have
              access to the source. The{" "}
              <Link
                href="/parsers/syslog"
                className="text-[var(--color-key)] hover:underline"
              >
                syslog parser
              </Link>{" "}
              is for actual syslog records, including exported files and
              streams. It is not a substitute for journalctl&apos;s structured
              mode.
            </p>
          </div>
        </section>

        <section id="jc-rs" className="scroll-mt-24">
          <h2 className="text-3xl">Where jc-rs is the right boundary</h2>
          <p className="mt-4 leading-relaxed text-[var(--color-muted)]">
            A schema-aware text parser is useful when the text cannot be
            avoided. That happens more often than a greenfield script suggests:
            old appliances, support bundles, captured incident output, and
            commands whose maintainers expose only a terminal format are all
            common in production environments.
          </p>

          <div className="mt-6 overflow-x-auto rounded-xl border bg-[var(--color-surface)]">
            <table className="w-full min-w-[44rem] border-collapse text-left text-sm">
              <thead className="bg-[var(--color-sunk)] font-mono text-[11px] tracking-wide text-[var(--color-faint)] uppercase">
                <tr>
                  <th scope="col" className="px-4 py-3 font-medium">
                    Input you actually have
                  </th>
                  <th scope="col" className="px-4 py-3 font-medium">
                    Use
                  </th>
                  <th scope="col" className="px-4 py-3 font-medium">
                    Reason
                  </th>
                </tr>
              </thead>
              <tbody>
                <CommandRow
                  input="Current ip address or route state"
                  use="ip -j …"
                  reason="The producer owns a documented JSON mode."
                />
                <CommandRow
                  input="Current block-device inventory"
                  use="lsblk -J -o …"
                  reason="Native JSON plus explicit columns is the strongest contract."
                />
                <CommandRow
                  input="Current systemd journal entries"
                  use="journalctl -o json"
                  reason="It retains journal fields before display formatting."
                />
                <CommandRow
                  input="Saved legacy lsblk table"
                  use="jc-rs --lsblk"
                  reason="You cannot rerun the producer, so parse the artifact you have."
                />
                <CommandRow
                  input="systemctl unit-list table"
                  use="jc-rs --systemctl"
                  reason="The matching parser gives the known table a typed JSON boundary."
                />
                <CommandRow
                  input="ps, df, or ss text from supported systems"
                  use="matching jc-rs parser"
                  reason="Use one parser per command grammar; do not infer arbitrary columns."
                />
              </tbody>
            </table>
          </div>

          <CodeBlock label="Text-only systemctl pipeline">
            {`LC_ALL=C SYSTEMD_COLORS=0 \\
  systemctl --all --no-pager --full --plain |
  jc-rs --systemctl |
  jq 'map(select(.active == "failed"))'`}
          </CodeBlock>
          <p className="mt-4 text-sm leading-relaxed text-[var(--color-faint)]">
            The command, options, locale, and parser name together define the
            input contract. See{" "}
            <Link
              href="/guides/parsing-command-output-reliably"
              className="text-[var(--color-key)] underline-offset-4 hover:underline"
            >
              how to test command-output parsers across locale, width, and
              version changes
            </Link>
            .
          </p>
        </section>

        <section id="contract" className="scroll-mt-24">
          <h2 className="text-3xl">JSON syntax is not your schema contract</h2>
          <p className="mt-4 leading-relaxed text-[var(--color-muted)]">
            Switching to native JSON prevents column-splitting bugs, but it does
            not promise that a field will exist forever or keep the same type.
            Record the producer version, request explicit fields where possible,
            and assert the minimum shape your consumer needs.
          </p>
          <CodeBlock label="Fail if the lsblk contract changes">
            {`lsblk --json --tree --output NAME,TYPE,SIZE,MOUNTPOINTS |
  jq -e '
    select(
      (.blockdevices | type == "array") and
      all(.blockdevices[];
        (.name | type == "string") and
        (.type | type == "string") and
        (.mountpoints | type == "array")
      )
    )
  ' > block-devices.json`}
          </CodeBlock>
          <p className="mt-4 leading-relaxed text-[var(--color-muted)]">
            Run that assertion in CI against every supported distribution image.
            When you choose jc-rs, make the same kind of assertion against the
            fixture-derived schema shown on its parser page. The JSON producer
            changes, but the consumer still deserves a checked contract.
          </p>
        </section>

        <section
          aria-labelledby="sources"
          className="rounded-xl border bg-[var(--color-surface)] p-6"
        >
          <h2 id="sources" className="text-2xl">
            Upstream references
          </h2>
          <p className="mt-3 text-sm leading-relaxed text-[var(--color-muted)]">
            Flag behavior above is grounded in the upstream manuals: the{" "}
            <a
              href="https://man7.org/linux/man-pages/man8/ip.8.html"
              className="text-[var(--color-key)] underline-offset-4 hover:underline"
            >
              iproute2 ip(8) manual
            </a>
            ,{" "}
            <a
              href="https://man7.org/linux/man-pages/man8/lsblk.8.html"
              className="text-[var(--color-key)] underline-offset-4 hover:underline"
            >
              util-linux lsblk(8) manual
            </a>
            , and{" "}
            <a
              href="https://www.freedesktop.org/software/systemd/man/latest/journalctl.html"
              className="text-[var(--color-key)] underline-offset-4 hover:underline"
            >
              systemd journalctl documentation
            </a>
            . jc-rs behavior is checked against its repository fixtures.
          </p>
          <div className="mt-5 grid gap-3 sm:grid-cols-2">
            <RelatedLink
              href="/guides/bash-jc-rs-jq"
              title="Bash, jc-rs, and jq"
            >
              Build the rest of the pipeline without hiding upstream failures.
            </RelatedLink>
            <RelatedLink href="/parsers/lsblk" title="lsblk parser and example">
              Inspect the schema used when native JSON is unavailable.
            </RelatedLink>
            <RelatedLink href="/parsers/systemctl" title="systemctl parser">
              Convert a supported unit-list table into records.
            </RelatedLink>
            <RelatedLink href="/install" title="Install jc-rs">
              Add the fallback parser as one static binary.
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

function DecisionRow({
  rank,
  choice,
  when,
}: {
  rank: string;
  choice: string;
  when: string;
}) {
  return (
    <div className="grid gap-2 border-b px-5 py-4 last:border-b-0 sm:grid-cols-[2rem_12rem_1fr]">
      <span className="font-mono text-xs text-[var(--color-faint)]">
        {rank}
      </span>
      <p className="text-sm font-medium text-[var(--color-ink)]">{choice}</p>
      <p className="text-sm leading-relaxed text-[var(--color-muted)]">
        {when}
      </p>
    </div>
  );
}

function CommandRow({
  input,
  use,
  reason,
}: {
  input: string;
  use: string;
  reason: string;
}) {
  return (
    <tr className="border-t align-top">
      <th scope="row" className="px-4 py-3 font-medium text-[var(--color-ink)]">
        {input}
      </th>
      <td className="px-4 py-3 font-mono text-xs text-[var(--color-key)]">
        {use}
      </td>
      <td className="px-4 py-3 leading-6 text-[var(--color-muted)]">
        {reason}
      </td>
    </tr>
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
