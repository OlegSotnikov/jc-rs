import type { Metadata } from "next";
import Link from "next/link";
import type { ReactNode } from "react";
import { site } from "@/lib/site";

const canonical = "/guides/curl-headers-to-json";
const title = "curl headers to JSON: HEAD, verbose traces, and redirects";
const description =
  "Convert curl response headers to JSON without mixing in the body. Covers --head, --dump-header, verbose stderr, repeated headers, redirects, and jc-rs.";
const published = "2026-08-11";

export const metadata: Metadata = {
  title,
  description,
  alternates: { canonical },
  openGraph: {
    type: "article",
    title,
    description,
    url: site.origin + canonical,
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
  url: site.origin + canonical,
  mainEntityOfPage: site.origin + canonical,
  datePublished: published,
  dateModified: published,
  articleSection: "Command-line guides",
  author: { "@type": "Person", name: site.author, url: site.authorUrl },
  publisher: { "@type": "Organization", name: site.name, url: site.origin },
  about: [
    "curl headers to JSON",
    "curl HEAD request",
    "curl verbose output",
    "duplicate HTTP headers",
    "HTTP redirects",
  ],
};

const captureRows = [
  [
    "--head (-I)",
    "HEAD",
    "Response headers on stdout",
    "You genuinely want HEAD semantics and no body",
  ],
  [
    "--dump-header (-D)",
    "Unchanged",
    "Received response headers to a file or stream",
    "You need headers from the real GET, POST, or other request",
  ],
  [
    "--verbose (-v)",
    "Unchanged",
    "Request and response trace on stderr; body on stdout",
    "You need both sides of the exchange for debugging",
  ],
] as const;

export default function CurlHeadersToJsonGuide() {
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
        <span>curl headers to JSON</span>
      </nav>

      <header className="mt-5 max-w-4xl">
        <p className="font-mono text-xs tracking-wide text-[var(--color-muted)] uppercase">
          HTTP pipeline guide
        </p>
        <h1 className="mt-3 max-w-4xl text-4xl sm:text-5xl">
          Convert curl headers to JSON without mixing in the body
        </h1>
        <p className="mt-5 max-w-3xl text-lg leading-relaxed text-[var(--color-muted)]">
          Choose the curl capture mode before choosing the parser.{" "}
          <code className="font-mono text-base">--head</code> sends an HTTP HEAD
          request; <code className="font-mono text-base">--verbose</code> traces
          an otherwise normal request to stderr;{" "}
          <code className="font-mono text-base">--dump-header</code> records
          received response headers without changing the method. jc-rs turns the
          resulting message blocks into typed JSON while keeping redirects and
          recognized repeated headers visible.
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
          <time dateTime={published}>August 11, 2026</time> · Output shapes
          verified against the jc-rs curl and HTTP-header fixtures
        </p>
      </header>

      <div
        aria-label="curl header processing pipeline"
        className="mt-9 grid overflow-hidden rounded-xl border bg-[var(--color-surface)] sm:grid-cols-[1fr_auto_1fr_auto_1fr_auto_1fr]"
      >
        <Stage label="Request" value="curl" />
        <Arrow />
        <Stage label="Capture" value="header blocks" />
        <Arrow />
        <Stage label="Parse" value="jc-rs --curl-head" accent />
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
          <a href="#capture" className="hover:text-[var(--color-key)]">
            Choose the curl capture mode
          </a>
          <a href="#head" className="hover:text-[var(--color-key)]">
            Parse a HEAD response
          </a>
          <a href="#real-method" className="hover:text-[var(--color-key)]">
            Keep GET or POST semantics
          </a>
          <a href="#verbose" className="hover:text-[var(--color-key)]">
            Capture curl -v correctly
          </a>
          <a href="#duplicates" className="hover:text-[var(--color-key)]">
            Repeated headers
          </a>
          <a href="#redirects" className="hover:text-[var(--color-key)]">
            Redirect chains
          </a>
          <a href="#schema" className="hover:text-[var(--color-key)]">
            jc-rs output schema
          </a>
          <a href="#failure" className="hover:text-[var(--color-key)]">
            Failures and sensitive data
          </a>
        </div>
      </nav>

      <div className="mt-14 max-w-3xl space-y-16">
        <section id="capture" className="scroll-mt-24">
          <h2 className="text-3xl">
            Three curl options answer different questions
          </h2>
          <p className="mt-4 leading-relaxed text-[var(--color-muted)]">
            “Show me the headers” is ambiguous. A server may handle HEAD
            differently from GET, and a verbose trace contains the outgoing
            request as well as every incoming response. Pick the row that
            matches what you need to observe.
          </p>
          <div className="mt-6 overflow-x-auto rounded-xl border bg-[var(--color-surface)]">
            <table className="w-full min-w-[49rem] border-collapse text-left text-sm">
              <thead className="bg-[var(--color-sunk)] font-mono text-[11px] tracking-wide text-[var(--color-faint)] uppercase">
                <tr>
                  <th scope="col" className="px-4 py-3 font-medium">
                    Option
                  </th>
                  <th scope="col" className="px-4 py-3 font-medium">
                    Method
                  </th>
                  <th scope="col" className="px-4 py-3 font-medium">
                    Captured data
                  </th>
                  <th scope="col" className="px-4 py-3 font-medium">
                    Use it when
                  </th>
                </tr>
              </thead>
              <tbody>
                {captureRows.map(([option, method, data, use]) => (
                  <tr key={option} className="border-t align-top">
                    <th
                      scope="row"
                      className="px-4 py-3 font-mono text-xs text-[var(--color-key)]"
                    >
                      {option}
                    </th>
                    <td className="px-4 py-3">{method}</td>
                    <td className="px-4 py-3 leading-6 text-[var(--color-muted)]">
                      {data}
                    </td>
                    <td className="px-4 py-3 leading-6 text-[var(--color-muted)]">
                      {use}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
          <p className="mt-4 text-sm leading-relaxed text-[var(--color-faint)]">
            curl&apos;s <code className="font-mono">--json</code> option is
            unrelated: it is shorthand for sending a JSON request body and
            content headers. It does not convert response headers or the
            response body to JSON.
          </p>
        </section>

        <section id="head" className="scroll-mt-24">
          <h2 className="text-3xl">Parse a real HEAD response from stdout</h2>
          <p className="mt-4 leading-relaxed text-[var(--color-muted)]">
            For HTTP, <code className="font-mono text-sm">--head</code> (or{" "}
            <code className="font-mono text-sm">-I</code>) asks curl to issue
            HEAD. The response has headers but no response body, so stdout is
            already a clean parser input.
          </p>
          <CodeBlock
            label="HEAD response to JSON"
            lines={[
              "#!/usr/bin/env bash",
              "set -Eeuo pipefail",
              "",
              "curl --silent --show-error --head https://example.com/ |",
              "  jc-rs --curl-head |",
              "  jq '.[0] | {",
              "    status: ._response_status,",
              '    content_type: .["content-type"],',
              '    content_length: .["content-length"]',
              "  }'",
            ]}
          />
          <p className="mt-4 leading-relaxed text-[var(--color-muted)]">
            This measures the HEAD endpoint, not “GET without downloading the
            body.” That distinction matters when an application generates
            headers dynamically, a CDN handles methods differently, or an origin
            has incomplete HEAD support.
          </p>
        </section>

        <section id="real-method" className="scroll-mt-24">
          <h2 className="text-3xl">
            Use --dump-header when the actual method matters
          </h2>
          <p className="mt-4 leading-relaxed text-[var(--color-muted)]">
            <code className="font-mono text-sm">--dump-header</code> (or{" "}
            <code className="font-mono text-sm">-D</code>) writes received
            protocol headers to a destination you choose. It does not turn GET
            into HEAD. Keep the body and headers in separate files, then parse
            the header artifact after curl succeeds.
          </p>
          <CodeBlock
            label="Headers and body from the same GET"
            lines={[
              "url=https://example.com/",
              "",
              "curl --silent --show-error \\",
              "  --dump-header response.headers \\",
              "  --output response.body \\",
              '  "$url"',
              "",
              "jc-rs --http-headers < response.headers > response-headers.json",
            ]}
          />
          <p className="mt-4 leading-relaxed text-[var(--color-muted)]">
            A clean header file can use the lower-level{" "}
            <Link
              href="/parsers/http-headers"
              className="text-[var(--color-key)] underline-offset-4 hover:underline"
            >
              HTTP headers parser
            </Link>
            . The{" "}
            <Link
              href="/parsers/curl-head"
              className="text-[var(--color-key)] underline-offset-4 hover:underline"
            >
              curl head parser
            </Link>{" "}
            accepts it too; its extra job is stripping verbose prefixes and
            informational lines.
          </p>
          <div className="mt-5 rounded-xl border-l-2 border-[var(--color-num)] bg-[var(--color-surface)] px-5 py-4">
            <p className="text-sm leading-relaxed text-[var(--color-muted)]">
              Avoid <code className="font-mono">curl --include</code> as parser
              input when you also need the body. It places headers and body on
              the same stdout stream; a body can contain text that looks exactly
              like another HTTP message.
            </p>
          </div>
        </section>

        <section id="verbose" className="scroll-mt-24">
          <h2 className="text-3xl">curl -v is a trace on stderr</h2>
          <p className="mt-4 leading-relaxed text-[var(--color-muted)]">
            Verbose mode does not mean “print response headers to stdout.” curl
            writes the body to stdout as usual and sends protocol trace lines to
            stderr. Outgoing lines begin with{" "}
            <code className="font-mono text-sm">&gt; </code>, incoming lines
            with <code className="font-mono text-sm">&lt; </code>, and
            connection notes with <code className="font-mono text-sm">* </code>.
          </p>
          <CodeBlock
            label="Wrong stream: this feeds the response body to jc-rs"
            lines={["curl --verbose https://example.com/ | jc-rs --curl-head"]}
          />
          <CodeBlock
            label="Capture, check, then parse the verbose exchange"
            lines={[
              "#!/usr/bin/env bash",
              "set -Eeuo pipefail",
              "",
              "url=https://example.com/",
              "trace_file=$(mktemp)",
              "trap 'rm -f -- \"$trace_file\"' EXIT",
              "",
              "if ! curl --silent --show-error --verbose \\",
              "  --output /dev/null \\",
              '  "$url" 2>"$trace_file"; then',
              '  cat "$trace_file" >&2',
              "  exit 1",
              "fi",
              "",
              'jc-rs --curl-head <"$trace_file" |',
              "  jq 'map(select(._type == \"response\"))'",
            ]}
          />
          <p className="mt-4 leading-relaxed text-[var(--color-muted)]">
            jc-rs removes curl&apos;s incoming and outgoing prefixes, ignores
            recognized connection and timing lines, and delegates the remaining
            messages to the HTTP header parser. A verbose exchange can therefore
            produce request and response objects in wire order.
          </p>
        </section>

        <section id="duplicates" className="scroll-mt-24">
          <h2 className="text-3xl">
            Repeated headers need header-specific rules
          </h2>
          <p className="mt-4 leading-relaxed text-[var(--color-muted)]">
            JSON cannot retain two useful values under one key without choosing
            an array or another representation. Blind comma splitting is wrong
            too: separate <code className="font-mono text-sm">Set-Cookie</code>{" "}
            lines are distinct values, and a cookie expiry date itself contains
            a comma.
          </p>
          <CodeBlock
            label="Two cookies and two cache directives"
            lines={[
              "printf '%s\\r\\n' \\",
              "  'HTTP/1.1 302 Found' \\",
              "  'Location: https://example.com/final' \\",
              "  'Set-Cookie: theme=dark; Path=/' \\",
              "  'Set-Cookie: session=abc123; Path=/; HttpOnly' \\",
              "  'Cache-Control: no-store' \\",
              "  'Cache-Control: private' \\",
              "  '' |",
              "  jc-rs --curl-head |",
              "  jq '.[0] | {",
              "    status: ._response_status,",
              "    location,",
              '    cookies: .["set-cookie"],',
              '    cache_control: .["cache-control"]',
              "  }'",
            ]}
          />
          <CodeBlock
            label="Result"
            lines={[
              "{",
              '  "status": 302,',
              '  "location": "https://example.com/final",',
              '  "cookies": [',
              '    "theme=dark; Path=/",',
              '    "session=abc123; Path=/; HttpOnly"',
              "  ],",
              '  "cache_control": ["no-store", "private"]',
              "}",
            ]}
          />
          <p className="mt-5 leading-relaxed text-[var(--color-muted)]">
            The current schema accumulates{" "}
            <code className="font-mono text-sm">set-cookie</code>,{" "}
            <code className="font-mono text-sm">cookie</code>, and
            content-security-policy fields without comma splitting. It
            aggregates and splits a maintained set of list-valued headers such
            as <code className="font-mono text-sm">cache-control</code>,{" "}
            <code className="font-mono text-sm">vary</code>, and{" "}
            <code className="font-mono text-sm">www-authenticate</code>.
          </p>
          <div className="mt-5 rounded-xl border-l-2 border-[var(--color-num)] bg-[var(--color-surface)] px-5 py-4">
            <p className="text-sm leading-relaxed text-[var(--color-muted)]">
              Only the maintained header sets get this treatment. For an
              ordinary header outside those sets, a later occurrence currently
              replaces the earlier one. If an extension header may repeat and
              every instance matters, retain the raw artifact and add a tested
              rule before treating the JSON as lossless.
            </p>
          </div>
        </section>

        <section id="redirects" className="scroll-mt-24">
          <h2 className="text-3xl">A redirect chain is several responses</h2>
          <p className="mt-4 leading-relaxed text-[var(--color-muted)]">
            Without <code className="font-mono text-sm">--location</code>, curl
            stops at the first redirect. With it, header capture contains a
            response block for every hop. Keep those blocks separate: merging
            would attach an early response&apos;s cookies or cache policy to the
            final resource.
          </p>
          <CodeBlock
            label="Follow a GET and retain every response status"
            lines={[
              "#!/usr/bin/env bash",
              "set -Eeuo pipefail",
              "",
              "curl --silent --show-error --location \\",
              "  --dump-header - \\",
              "  --output /dev/null \\",
              "  http://example.com/ |",
              "  jc-rs --curl-head |",
              "  jq 'map(",
              '    select(._type == "response")',
              "    | {",
              "        status: ._response_status,",
              "        location: (.location // null),",
              '        content_type: (.["content-type"] // null)',
              "      }",
              "  )'",
            ]}
          />
          <p className="mt-4 leading-relaxed text-[var(--color-muted)]">
            Combining{" "}
            <code className="font-mono text-sm">--head --location</code>{" "}
            produces a chain of HEAD requests; use it only when HEAD behavior is
            the question. Proxies, authentication handshakes, and informational
            1xx responses can also add message boundaries, so select the final
            response instead of assuming index zero.
          </p>
          <CodeBlock
            label="Retain the chain and name the final response"
            lines={[
              "jq '{",
              '  chain: [ .[] | select(._type == "response") ],',
              '  final: ([ .[] | select(._type == "response") ] | last)',
              "}' response-exchange.json",
            ]}
          />
        </section>

        <section id="schema" className="scroll-mt-24">
          <h2 className="text-3xl">Use the exact jc-rs header field names</h2>
          <p className="mt-4 leading-relaxed text-[var(--color-muted)]">
            Each HTTP message becomes one object. Header names are lowercase and
            remain kebab-cased. Use bracket syntax in jq for a hyphenated name:{" "}
            <code className="font-mono text-sm">
              .[&quot;content-type&quot;]
            </code>{" "}
            is a field lookup, while a bare hyphen can be parsed as subtraction.
          </p>
          <div className="mt-6 overflow-hidden rounded-xl border bg-[var(--color-surface)]">
            <SchemaRow
              field="_type"
              value="request or response"
              note="Distinguishes message kinds in a verbose trace"
            />
            <SchemaRow
              field="_request_method / _request_uri"
              value="string"
              note="Present on request objects"
            />
            <SchemaRow
              field="_response_status"
              value="number"
              note="HTTP status code on response objects"
            />
            <SchemaRow
              field="content-length, age"
              value="number when parseable"
              note="Selected numeric headers are typed"
            />
            <SchemaRow
              field="set-cookie"
              value="array"
              note="Each cookie header stays separate"
            />
            <SchemaRow
              field="date_epoch_utc"
              value="number"
              note="Added beside a recognized HTTP date"
            />
          </div>
          <p className="mt-4 text-sm leading-relaxed text-[var(--color-faint)]">
            Inspect the fixture-backed example on the{" "}
            <Link
              href="/parsers/http-headers"
              className="text-[var(--color-key)] underline-offset-4 hover:underline"
            >
              HTTP headers parser page
            </Link>{" "}
            before writing a downstream jq contract.
          </p>
        </section>

        <section id="failure" className="scroll-mt-24">
          <h2 className="text-3xl">
            Separate HTTP status, process failure, and secrets
          </h2>
          <p className="mt-4 leading-relaxed text-[var(--color-muted)]">
            By default, an HTTP 404 can still be a successful curl transfer. The
            parsed <code className="font-mono text-sm">_response_status</code>{" "}
            reports the HTTP result; curl&apos;s process status reports whether
            the transfer completed under its options. Check both when the job
            cares about both.
          </p>
          <CodeBlock
            label="Require a completed transfer and a 2xx final response"
            lines={[
              "set -Eeuo pipefail",
              "",
              "curl --silent --show-error --location \\",
              "  --dump-header - \\",
              "  --output /dev/null \\",
              "  https://example.com/ |",
              "  jc-rs --curl-head |",
              "  jq -e '",
              '    [ .[] | select(._type == "response") ]',
              "    | last",
              "    | select(._response_status >= 200 and ._response_status < 300)",
              "  ' > final-response.json",
            ]}
          />
          <p className="mt-5 leading-relaxed text-[var(--color-muted)]">
            Verbose traces and parsed JSON can contain Authorization, Cookie,
            Set-Cookie, signed URLs, and internal hostnames. Protect raw
            captures and delete sensitive fields before sending JSON to logs,
            tickets, or analytics.
          </p>
          <CodeBlock
            label="Redact common credentials before logging"
            lines={[
              "jc-rs --curl-head < curl.trace |",
              "  jq 'map(del(",
              "    .authorization,",
              "    .cookie,",
              '    .["set-cookie"],',
              '    .["proxy-authorization"]',
              "  ))' > safe-for-review.json",
            ]}
          />
        </section>

        <section
          aria-labelledby="sources"
          className="rounded-xl border bg-[var(--color-surface)] p-6"
        >
          <h2 id="sources" className="text-2xl">
            curl manuals and parser source
          </h2>
          <p className="mt-3 text-sm leading-relaxed text-[var(--color-muted)]">
            Capture semantics come from the official{" "}
            <a
              href="https://curl.se/docs/manpage.html"
              className="text-[var(--color-key)] underline-offset-4 hover:underline"
            >
              curl command-line manual
            </a>{" "}
            and{" "}
            <a
              href="https://curl.se/docs/httpscripting.html"
              className="text-[var(--color-key)] underline-offset-4 hover:underline"
            >
              curl HTTP scripting guide
            </a>
            . Prefix removal, boundaries, conversions, and repeated-header
            behavior are checked against the{" "}
            <a
              href={
                site.repo +
                "/blob/master/crates/jc-rs-parsers/src/network/curl_head.rs"
              }
              className="text-[var(--color-key)] underline-offset-4 hover:underline"
            >
              curl parser
            </a>{" "}
            and{" "}
            <a
              href={
                site.repo +
                "/blob/master/crates/jc-rs-parsers/src/network/http_headers.rs"
              }
              className="text-[var(--color-key)] underline-offset-4 hover:underline"
            >
              HTTP header parser
            </a>{" "}
            source.
          </p>
          <div className="mt-5 grid gap-3 sm:grid-cols-2">
            <RelatedLink href="/parsers/curl-head" title="curl head parser">
              Inspect coverage and use the browser workbench.
            </RelatedLink>
            <RelatedLink
              href="/parsers/http-headers"
              title="HTTP headers parser"
            >
              Parse a clean request or response block.
            </RelatedLink>
            <RelatedLink
              href="/guides/bash-jc-rs-jq"
              title="Bash, jc-rs, and jq"
            >
              Preserve quoting, stderr, and upstream status.
            </RelatedLink>
            <RelatedLink
              href="/guides/parsing-command-output-reliably"
              title="Reliable command parsing"
            >
              Test errors and schemas as one contract.
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
        className={
          accent
            ? "mt-1 block font-mono text-sm text-[var(--color-key)]"
            : "mt-1 block font-mono text-sm"
        }
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

function SchemaRow({
  field,
  value,
  note,
}: {
  field: string;
  value: string;
  note: string;
}) {
  return (
    <div className="grid gap-2 border-b px-5 py-4 last:border-b-0 sm:grid-cols-[13rem_9rem_1fr]">
      <code className="overflow-x-auto font-mono text-xs text-[var(--color-key)]">
        {field}
      </code>
      <p className="text-sm text-[var(--color-ink)]">{value}</p>
      <p className="text-sm leading-relaxed text-[var(--color-muted)]">
        {note}
      </p>
    </div>
  );
}

function CodeBlock({ label, lines }: { label: string; lines: string[] }) {
  return (
    <figure className="mt-5 overflow-hidden rounded-xl border bg-[var(--color-sunk)]">
      <figcaption className="border-b bg-[var(--color-surface)] px-4 py-2 font-mono text-[10px] tracking-wide text-[var(--color-faint)] uppercase">
        {label}
      </figcaption>
      <pre className="overflow-x-auto p-4 font-mono text-xs leading-relaxed">
        <code>{lines.join("\n")}</code>
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
