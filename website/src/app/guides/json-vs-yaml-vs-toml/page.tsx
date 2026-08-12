import type { Metadata } from "next";
import Link from "next/link";
import type { ReactNode } from "react";
import { site } from "@/lib/site";

const slug = "/guides/json-vs-yaml-vs-toml";
const pageUrl = site.origin + slug;
const published = "2026-08-11";

export const metadata: Metadata = {
  title: "JSON vs YAML vs TOML: Which Format Fits?",
  description:
    "Compare JSON, YAML, TOML and XML for configuration, CLI pipelines and interchange: comments, types, schemas, streaming and conversion trade-offs.",
  alternates: { canonical: slug },
  openGraph: {
    siteName: site.name,
    type: "article",
    title: "JSON vs YAML vs TOML: which format fits?",
    description:
      "A practical format comparison for configuration and command-line work, including where XML remains the right model.",
    url: pageUrl,
    publishedTime: published,
    modifiedTime: published,
    authors: [site.authorUrl],
    images: [site.socialImage],
  },
  twitter: {
    card: "summary_large_image",
    title: "JSON vs YAML vs TOML: which format fits?",
    description:
      "A practical format comparison for configuration and command-line work, including where XML remains the right model.",
    images: [site.socialImage],
  },
};

const articleJsonLd = {
  "@context": "https://schema.org",
  "@type": "Article",
  headline:
    "JSON vs YAML vs TOML: choosing a format for config and CLI pipelines",
  description:
    "A practical comparison of JSON, YAML, TOML and XML across human editing, types, comments, validation, streaming and conversion.",
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
  about: [
    "JSON",
    "YAML",
    "TOML",
    "XML",
    "configuration files",
    "CLI pipelines",
  ],
};

const comparisonRows = [
  {
    dimension: "Best default for",
    json: "APIs, generated data, CLI interchange",
    yaml: "Large human-edited config and ecosystem manifests",
    toml: "Application and project config",
    xml: "Document trees, established protocols and schema-led systems",
  },
  {
    dimension: "Comments",
    json: "No",
    yaml: "# comments",
    toml: "# comments",
    xml: "<!-- comments -->",
  },
  {
    dimension: "Core model",
    json: "Values, arrays and string-keyed objects",
    yaml: "Mappings, sequences, scalars, tags and aliases",
    toml: "Key/value pairs, tables and arrays of tables",
    xml: "Ordered elements, attributes and text",
  },
  {
    dimension: "Null value",
    json: "Native",
    yaml: "Native",
    toml: "No native null",
    xml: "A convention or schema decision",
  },
  {
    dimension: "Date/time values",
    json: "Usually strings by convention",
    yaml: "Depends on version and parser schema",
    toml: "Native date and time types",
    xml: "Text unless a schema assigns a type",
  },
  {
    dimension: "Several documents or records",
    json: "Needs an array or external framing",
    yaml: "Supports document separators",
    toml: "One configuration document",
    xml: "One root document; incremental parsers exist",
  },
  {
    dimension: "Repeated names",
    json: "Duplicate object names are unsafe",
    yaml: "Mapping keys must be unique",
    toml: "Keys and tables cannot be redefined",
    xml: "Repeated sibling elements are native",
  },
  {
    dimension: "Common validation route",
    json: "JSON Schema or application rules",
    yaml: "Often JSON Schema after loading",
    toml: "Application or ecosystem-specific schema",
    xml: "XSD, RELAX NG or application rules",
  },
] as const;

export default function FormatComparisonGuide() {
  return (
    <div className="mx-auto max-w-6xl px-5 py-14">
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
        <span>json, yaml, toml and xml</span>
      </nav>

      <article>
        <header className="mt-5">
          <p className="font-mono text-xs tracking-wide text-[var(--color-muted)] uppercase">
            Format guide
          </p>
          <h1 className="mt-3 max-w-4xl text-4xl sm:text-5xl">
            JSON vs YAML vs TOML: where XML still fits
          </h1>
          <p className="mt-5 max-w-3xl text-lg leading-8 text-[var(--color-muted)]">
            JSON is the safest default between programs. YAML suits
            configuration that people edit and an ecosystem already expects.
            TOML is a tighter fit for application config with explicit types and
            tables. XML remains strong when the data is really a document, not
            merely an object written with angle brackets.
          </p>
          <div className="mt-5 flex flex-wrap items-center gap-x-3 gap-y-1 font-mono text-xs text-[var(--color-faint)]">
            <a href={site.authorUrl} className="hover:text-[var(--color-ink)]">
              {site.author}
            </a>
            <span aria-hidden="true">·</span>
            <time dateTime={published}>August 11, 2026</time>
            <span aria-hidden="true">·</span>
            <span>11 min read</span>
          </div>
        </header>

        <section aria-labelledby="short-answer" className="mt-10">
          <div className="rounded-xl border bg-[var(--color-surface)] p-5 sm:p-6">
            <div className="flex flex-wrap items-baseline justify-between gap-2">
              <h2 id="short-answer" className="text-xl">
                The short answer
              </h2>
              <p className="font-mono text-[11px] tracking-wide text-[var(--color-faint)] uppercase">
                choose per boundary
              </p>
            </div>
            <div className="mt-5 grid gap-px overflow-hidden rounded-lg border bg-[var(--color-rule)] sm:grid-cols-2 lg:grid-cols-4">
              <FormatSignal name="JSON" color="key">
                Pipes, APIs, generated data
              </FormatSignal>
              <FormatSignal name="YAML" color="str">
                Large human-edited config
              </FormatSignal>
              <FormatSignal name="TOML" color="num">
                Application and project config
              </FormatSignal>
              <FormatSignal name="XML" color="punct">
                Documents and established contracts
              </FormatSignal>
            </div>
            <p className="mt-5 max-w-3xl text-sm leading-6 text-[var(--color-muted)]">
              A project does not need one winner. It can read TOML
              configuration, accept YAML manifests, speak JSON over an API, and
              consume an XML protocol without contradiction. The boundary should
              decide the format.
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
                <a href="#matrix" className="hover:text-[var(--color-ink)]">
                  Comparison matrix
                </a>
              </li>
              <li>
                <a href="#syntax" className="hover:text-[var(--color-ink)]">
                  Same data, four formats
                </a>
              </li>
              <li>
                <a href="#pairs" className="hover:text-[var(--color-ink)]">
                  Head-to-head choices
                </a>
              </li>
              <li>
                <a href="#decision" className="hover:text-[var(--color-ink)]">
                  Decision path
                </a>
              </li>
              <li>
                <a href="#convert" className="hover:text-[var(--color-ink)]">
                  Convert with jc-rs
                </a>
              </li>
              <li>
                <a href="#loss" className="hover:text-[var(--color-ink)]">
                  What conversion loses
                </a>
              </li>
              <li>
                <a href="#faq" className="hover:text-[var(--color-ink)]">
                  Common questions
                </a>
              </li>
            </ol>
          </aside>

          <div className="min-w-0 max-w-4xl">
            <section id="matrix" className="scroll-mt-24">
              <h2 className="text-3xl">JSON, YAML, TOML and XML compared</h2>
              <p className="mt-3 max-w-3xl leading-7 text-[var(--color-muted)]">
                The syntax is the visible part. The harder differences are the
                data model, how implementations assign types, and whether a
                document survives a round trip.
              </p>
              <div className="mt-6 overflow-x-auto rounded-xl border bg-[var(--color-surface)]">
                <table className="w-full min-w-[64rem] border-collapse text-left text-sm">
                  <thead className="bg-[var(--color-sunk)] font-mono text-[11px] tracking-wide text-[var(--color-faint)] uppercase">
                    <tr>
                      <th scope="col" className="px-4 py-3 font-medium">
                        Question
                      </th>
                      <th
                        scope="col"
                        className="px-4 py-3 font-medium text-[var(--color-key)]"
                      >
                        JSON
                      </th>
                      <th
                        scope="col"
                        className="px-4 py-3 font-medium text-[var(--color-str)]"
                      >
                        YAML
                      </th>
                      <th
                        scope="col"
                        className="px-4 py-3 font-medium text-[var(--color-num)]"
                      >
                        TOML
                      </th>
                      <th scope="col" className="px-4 py-3 font-medium">
                        XML
                      </th>
                    </tr>
                  </thead>
                  <tbody>
                    {comparisonRows.map((row) => (
                      <tr key={row.dimension} className="border-t align-top">
                        <th
                          scope="row"
                          className="w-40 px-4 py-3 font-medium text-[var(--color-ink)]"
                        >
                          {row.dimension}
                        </th>
                        <td className="px-4 py-3 leading-6 text-[var(--color-muted)]">
                          {row.json}
                        </td>
                        <td className="px-4 py-3 leading-6 text-[var(--color-muted)]">
                          {row.yaml}
                        </td>
                        <td className="px-4 py-3 leading-6 text-[var(--color-muted)]">
                          {row.toml}
                        </td>
                        <td className="px-4 py-3 leading-6 text-[var(--color-muted)]">
                          {row.xml}
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
              <p className="mt-3 text-sm leading-6 text-[var(--color-faint)]">
                “Supports a type” means the format can express it. Your parser,
                schema, and target language still decide the in-memory
                representation.
              </p>
            </section>

            <section id="syntax" className="mt-14 scroll-mt-24">
              <h2 className="text-3xl">The same service in four grammars</h2>
              <p className="mt-3 max-w-3xl leading-7 text-[var(--color-muted)]">
                These snippets express similar intent, not byte-for-byte
                interchangeable data. The XML version makes that distinction
                visible: without a schema, its numbers and booleans are text,
                and repeated ports are elements rather than an array value.
              </p>
              <div className="mt-6 grid gap-4 xl:grid-cols-2">
                <CodeSample label="config.json">
                  {`{
  "service": {
    "name": "api",
    "ports": [8080, 8081],
    "debug": false
  }
}`}
                </CodeSample>
                <CodeSample label="config.yaml">
                  {`service:
  name: api
  ports:
    - 8080
    - 8081
  debug: false`}
                </CodeSample>
                <CodeSample label="config.toml">
                  {`[service]
name = "api"
ports = [8080, 8081]
debug = false`}
                </CodeSample>
                <CodeSample label="config.xml">
                  {`<config>
  <service>
    <name>api</name>
    <ports>
      <port>8080</port>
      <port>8081</port>
    </ports>
    <debug>false</debug>
  </service>
</config>`}
                </CodeSample>
              </div>
            </section>

            <section id="pairs" className="mt-14 scroll-mt-24">
              <h2 className="text-3xl">Where each pair differs</h2>

              <h3 className="mt-8 text-xl">
                YAML vs JSON: authoring comfort or interchange certainty
              </h3>
              <p className="mt-3 leading-7 text-[var(--color-muted)]">
                JSON has a small data model and near-universal library support.
                Its quotes and braces are noisy in hand-written files, but they
                leave relatively little room for a reader to wonder where a
                value begins. That makes JSON a sound default for APIs,
                generated artifacts, and stdout that another command will
                consume.
              </p>
              <p className="mt-3 leading-7 text-[var(--color-muted)]">
                YAML removes much of that punctuation and adds comments, block
                strings, anchors, aliases, tags, and multi-document files. Those
                features help in a large manifest, but they also mean “YAML
                support” is not one perfectly uniform behavior. Scalar typing
                can vary with the YAML version and library. Quote a value when
                it must stay a string, and test with the same parser used in
                production.
              </p>

              <h3 className="mt-8 text-xl">
                YAML vs TOML: two approaches to human-edited config
              </h3>
              <p className="mt-3 leading-7 text-[var(--color-muted)]">
                YAML mirrors deeply nested objects and long sequences compactly.
                TOML organizes configuration around named tables and key/value
                assignments. TOML is often easier to audit for a modest
                application config because strings are quoted and date/time
                values have explicit types. It has no native null, and deeply
                nested or highly repetitive structures can become table-heavy.
              </p>
              <p className="mt-3 leading-7 text-[var(--color-muted)]">
                Pick YAML when the surrounding platform already speaks YAML or
                the file benefits from its document features. Pick TOML when you
                own the application boundary and the configuration naturally
                looks like sections of settings. Choosing TOML only to avoid
                YAML is not a design requirement; choosing it because the data
                fits tables is.
              </p>

              <h3 className="mt-8 text-xl">
                TOML vs JSON: configuration or transport
              </h3>
              <p className="mt-3 leading-7 text-[var(--color-muted)]">
                TOML comments and table headers make it friendlier for
                configuration maintained in a repository. JSON wins when the
                file is generated, crossed between languages, or piped into
                existing tools. A common, clean split is TOML on disk for
                settings and JSON on stdout for results.
              </p>

              <h3 className="mt-8 text-xl">
                JSON vs XML: value tree or document tree
              </h3>
              <p className="mt-3 leading-7 text-[var(--color-muted)]">
                JSON models program values: objects, arrays, and scalars. XML
                models ordered elements containing attributes, child elements,
                and text. XML can represent mixed prose and markup, namespaces,
                and repeated ordered elements without pretending they are object
                properties. Mature schema systems are another reason an
                established XML contract may be worth keeping.
              </p>
              <p className="mt-3 leading-7 text-[var(--color-muted)]">
                For a new object-shaped API, JSON is usually easier to consume
                from a shell and a browser. For a document format or an existing
                XML protocol, converting everything to JSON at the boundary can
                discard distinctions the XML consumer needs. When comparing JSON
                vs YAML vs XML, first ask whether the source is configuration,
                interchange data, or an actual marked-up document.
              </p>
            </section>

            <section id="decision" className="mt-14 scroll-mt-24">
              <h2 className="text-3xl">A practical selection order</h2>
              <ol className="mt-6 space-y-5">
                <Decision number="1" title="Honor the external contract.">
                  If Kubernetes, a package manager, an API, or a business
                  partner requires a format, compatibility outweighs personal
                  syntax preferences.
                </Decision>
                <Decision
                  number="2"
                  title="Use JSON between programs unless another model is needed."
                >
                  It has broad tooling, a small conceptual surface, and works
                  naturally with jq and command-line pipelines.
                </Decision>
                <Decision
                  number="3"
                  title="For human-owned config, compare the shape."
                >
                  TOML is strong for sections of application settings. YAML is
                  strong for large, nested manifests and ecosystems built around
                  YAML. JSON is reasonable when the file is mostly generated.
                </Decision>
                <Decision
                  number="4"
                  title="Keep XML when document semantics are real."
                >
                  Attributes, mixed content, namespaces, element order, or an
                  existing schema are requirements, not cosmetic syntax.
                </Decision>
                <Decision
                  number="5"
                  title="Separate streaming from format preference."
                >
                  A continuous sequence of records needs framing. Use{" "}
                  <Link
                    href="/guides/ndjson-vs-json-vs-jsonl"
                    className="text-[var(--color-key)] underline-offset-4 hover:underline"
                  >
                    NDJSON or JSONL
                  </Link>{" "}
                  when each record is JSON and should move independently.
                </Decision>
              </ol>
            </section>

            <section id="convert" className="mt-14 scroll-mt-24">
              <h2 className="text-3xl">
                Convert YAML, TOML, and XML to JSON with jc-rs
              </h2>
              <p className="mt-3 leading-7 text-[var(--color-muted)]">
                JSON needs no jc-rs parser when it is already JSON. For the
                other three formats, choose the parser that matches the input
                and pipe the resulting JSON wherever it needs to go.
              </p>

              <div className="mt-6 space-y-6">
                <Converter
                  title="YAML to JSON"
                  href="/parsers/yaml"
                  parser="--yaml"
                  command="cat config.yaml | jc-rs --yaml --pretty"
                >
                  jc-rs returns an array of YAML documents even when the file
                  contains one document. That keeps single-document and{" "}
                  <code className="font-mono text-sm">---</code>-separated
                  inputs on the same output shape. Use{" "}
                  <code className="font-mono text-sm">jq &apos;.[0]&apos;</code>{" "}
                  only when your contract explicitly accepts the first document
                  and ignores the rest.
                </Converter>
                <Converter
                  title="TOML to JSON"
                  href="/parsers/toml"
                  parser="--toml"
                  command="cat config.toml | jc-rs --toml --pretty"
                >
                  TOML tables become JSON objects. For compatibility with jc, a
                  named top-level TOML date/time value becomes a Unix timestamp
                  at the original key plus an ISO string at a sibling key ending
                  in <code className="font-mono text-sm">_iso</code>. Check that
                  mapping before a round trip.
                </Converter>
                <Converter
                  title="XML to JSON"
                  href="/parsers/xml"
                  parser="--xml"
                  command="cat document.xml | jc-rs --xml --pretty"
                >
                  Attributes receive an{" "}
                  <code className="font-mono text-sm">@</code> prefix, mixed
                  text uses <code className="font-mono text-sm">#text</code>,
                  and repeated sibling elements become arrays. XML scalar text
                  remains JSON strings unless your application converts it
                  further.
                </Converter>
              </div>

              <p className="mt-6 leading-7 text-[var(--color-muted)]">
                All three parser pages include their accepted input and a real
                output example. If you do not have the binary yet, start with
                the{" "}
                <Link
                  href="/install"
                  className="text-[var(--color-key)] underline-offset-4 hover:underline"
                >
                  installation options
                </Link>
                .
              </p>
            </section>

            <section id="loss" className="mt-14 scroll-mt-24">
              <h2 className="text-3xl">
                Conversion is useful; round-tripping is a different promise
              </h2>
              <p className="mt-3 leading-7 text-[var(--color-muted)]">
                Converting a format into JSON gives downstream tools a common
                value model. It does not preserve every source-level choice.
                Treat the JSON as a derived representation unless you have
                tested the exact return trip.
              </p>
              <ul className="mt-6 space-y-4 text-[var(--color-muted)]">
                <li className="leading-7">
                  <strong className="font-medium text-[var(--color-ink)]">
                    Comments and layout disappear.
                  </strong>{" "}
                  JSON has nowhere standard to store YAML and TOML comments, XML
                  comments as editorial placement, quoting style, or the
                  author&apos;s whitespace.
                </li>
                <li className="leading-7">
                  <strong className="font-medium text-[var(--color-ink)]">
                    YAML can exceed JSON&apos;s model.
                  </strong>{" "}
                  Tags, aliases, non-string mapping keys, and library-specific
                  scalar resolution need a mapping decision before they become
                  JSON.
                </li>
                <li className="leading-7">
                  <strong className="font-medium text-[var(--color-ink)]">
                    TOML has types JSON lacks.
                  </strong>{" "}
                  Dates and times need an agreed string, number, or
                  companion-field representation; TOML also has no null to
                  receive a JSON null.
                </li>
                <li className="leading-7">
                  <strong className="font-medium text-[var(--color-ink)]">
                    XML is not a JSON object with different brackets.
                  </strong>{" "}
                  Attributes, sibling order, namespaces, mixed text, and
                  repeated elements require a convention. Two converters can
                  choose different, internally consistent shapes.
                </li>
                <li className="leading-7">
                  <strong className="font-medium text-[var(--color-ink)]">
                    Duplicate names are a data-quality problem.
                  </strong>{" "}
                  Do not rely on “last value wins.” Reject duplicate JSON object
                  names and YAML mapping keys at ingestion, and let TOML&apos;s
                  redefinition errors surface.
                </li>
              </ul>
            </section>

            <section id="faq" className="mt-14 scroll-mt-24">
              <h2 className="text-3xl">Common questions</h2>
              <div className="mt-6 divide-y rounded-xl border bg-[var(--color-surface)] px-5 sm:px-6">
                <Question title="Is YAML better than JSON for configuration?">
                  YAML is usually easier to annotate and can be concise for deep
                  structures. JSON is more uniform across parsers and better for
                  generated configuration. The application&apos;s ecosystem and
                  parser behavior matter more than punctuation.
                </Question>
                <Question title="When should I choose TOML instead of YAML?">
                  Choose TOML when you own the config format and it naturally
                  consists of named sections, scalar settings, and modest
                  arrays. Choose YAML when you need its richer document features
                  or must fit an existing YAML ecosystem.
                </Question>
                <Question title="Is JSON always better than XML for APIs?">
                  No. JSON is a practical default for object-shaped web and CLI
                  data. XML remains a sound choice when an existing contract,
                  namespaces, ordered document content, or schema tooling is
                  part of the requirement.
                </Question>
                <Question title="Which format is smallest or fastest?">
                  There is no honest universal ranking. Data shape, whitespace,
                  compression, parser implementation, and validation dominate.
                  Benchmark the exact documents and libraries on your path
                  rather than choosing from a generic claim.
                </Question>
                <Question title="Can I convert between all four without losing information?">
                  Not generally. JSON-shaped values convert readily, but
                  comments, source layout, YAML tags and aliases, TOML dates,
                  and XML document semantics need explicit conventions.
                  Conversion and lossless round-tripping are separate
                  requirements.
                </Question>
              </div>
            </section>

            <section className="mt-14 rounded-xl border bg-[var(--color-surface)] p-6 sm:p-7">
              <p className="font-mono text-[11px] tracking-wide text-[var(--color-key)] uppercase">
                Working rule
              </p>
              <h2 className="mt-3 text-2xl">
                Convert at the boundary, not in the source of truth
              </h2>
              <p className="mt-3 leading-7 text-[var(--color-muted)]">
                A service can keep maintainable YAML or TOML configuration and
                still emit compact JSON to command-line consumers. An XML
                contract should remain XML when its document model carries
                information the JSON mapping cannot preserve. jc-rs handles the
                conversion when a shell pipeline needs a predictable JSON view
                of those sources.
              </p>
              <div className="mt-5 flex flex-wrap gap-3">
                <Link
                  href="/parsers"
                  className="rounded-md border px-4 py-2 text-sm transition-colors hover:border-[var(--color-key)]"
                >
                  Browse file parsers
                </Link>
                <Link
                  href="/compare"
                  className="rounded-md border px-4 py-2 text-sm transition-colors hover:border-[var(--color-key)]"
                >
                  Compare jc-rs and jc
                </Link>
              </div>
            </section>
          </div>
        </div>
      </article>
    </div>
  );
}

function FormatSignal({
  name,
  color,
  children,
}: {
  name: string;
  color: "key" | "str" | "num" | "punct";
  children: ReactNode;
}) {
  const textColor = {
    key: "text-[var(--color-key)]",
    str: "text-[var(--color-str)]",
    num: "text-[var(--color-num)]",
    punct: "text-[var(--color-punct)]",
  }[color];

  return (
    <div className="bg-[var(--color-surface)] p-4">
      <p className={"font-mono text-sm font-semibold " + textColor}>{name}</p>
      <p className="mt-2 text-xs leading-5 text-[var(--color-muted)]">
        {children}
      </p>
    </div>
  );
}

function CodeSample({ label, children }: { label: string; children: string }) {
  return (
    <figure className="min-w-0 overflow-hidden rounded-lg border bg-[var(--color-sunk)]">
      <figcaption className="border-b px-4 py-2 font-mono text-[10px] tracking-wide text-[var(--color-faint)] uppercase">
        {label}
      </figcaption>
      <pre className="overflow-x-auto p-4 font-mono text-xs leading-relaxed">
        <code>{children}</code>
      </pre>
    </figure>
  );
}

function Decision({
  number,
  title,
  children,
}: {
  number: string;
  title: string;
  children: ReactNode;
}) {
  return (
    <li className="grid grid-cols-[2rem_minmax(0,1fr)] gap-3">
      <span className="font-mono text-sm text-[var(--color-key)]">
        {number.padStart(2, "0")}
      </span>
      <p className="leading-7 text-[var(--color-muted)]">
        <strong className="font-medium text-[var(--color-ink)]">{title}</strong>{" "}
        {children}
      </p>
    </li>
  );
}

function Converter({
  title,
  href,
  parser,
  command,
  children,
}: {
  title: string;
  href: string;
  parser: string;
  command: string;
  children: ReactNode;
}) {
  return (
    <section className="rounded-xl border bg-[var(--color-surface)] p-5">
      <div className="flex flex-wrap items-baseline justify-between gap-2">
        <h3 className="text-xl">{title}</h3>
        <Link
          href={href}
          className="font-mono text-xs text-[var(--color-key)] underline-offset-4 hover:underline"
        >
          {parser} reference
        </Link>
      </div>
      <pre className="mt-4 overflow-x-auto rounded-lg bg-[var(--color-sunk)] p-4 font-mono text-xs leading-relaxed">
        <code>{command}</code>
      </pre>
      <p className="mt-4 text-sm leading-6 text-[var(--color-muted)]">
        {children}
      </p>
    </section>
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
