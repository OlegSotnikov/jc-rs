import Link from "next/link";
import type { Metadata } from "next";
import { notFound, permanentRedirect } from "next/navigation";
import { Panes } from "@/components/Panes";
import { CopyLine } from "@/components/CopyLine";
import { ParserWorkbench } from "@/components/ParserWorkbench";
import { guides } from "@/lib/guides";
import { getParserSeo } from "@/lib/parser-seo";
import {
  fromSlug,
  parsers,
  platformLabel,
  slugOf,
  type Parser,
} from "@/lib/parsers";
import { site } from "@/lib/site";

type Params = { params: Promise<{ slug: string }> };

export function generateStaticParams() {
  return parsers.map((p) => ({ slug: slugOf(p) }));
}

export async function generateMetadata({ params }: Params): Promise<Metadata> {
  const { slug } = await params;
  const p = fromSlug(slug);
  if (!p) return {};
  const seo = getParserSeo(p.name);
  const canonicalSlug = slugOf(p);
  const fallbackTitle = genericParserTitle(p);
  const fallbackDescription = genericParserDescription(p);

  return {
    title: seo?.title ?? fallbackTitle,
    description: seo?.description ?? fallbackDescription,
    alternates: { canonical: `/parsers/${canonicalSlug}` },
    openGraph: {
      siteName: site.name,
      type: "website",
      title: seo?.title ?? fallbackTitle,
      description: seo?.description ?? fallbackDescription,
      url: `${site.origin}/parsers/${canonicalSlug}`,
      images: [site.socialImage],
    },
    twitter: {
      card: "summary_large_image",
      title: seo?.title ?? fallbackTitle,
      description: seo?.description ?? fallbackDescription,
      images: [site.socialImage],
    },
  };
}

export default async function ParserPage({ params }: Params) {
  const { slug } = await params;
  const p = fromSlug(slug);
  if (!p) notFound();
  const canonicalSlug = slugOf(p);
  if (slug !== canonicalSlug) permanentRedirect(`/parsers/${canonicalSlug}`);
  const seo = getParserSeo(p.name);

  const command = p.magic[0] ?? p.name;
  const procPath = p.magic[0]?.startsWith("/proc") ? procShellPath(p.magic[0]) : null;
  const shellInput = p.magic[0] ?? (seo ? seoShellInput(p.name) : "cat input.txt");
  const pipeline = procPath
    ? `jc-rs ${procPath}`
    : p.streaming
      ? `${shellInput} | jc-rs -u ${p.argument}`
      : `${shellInput} | jc-rs ${p.argument}`;
  const sampleInput = seo?.sampleInput ?? p.example?.input ?? "";
  const sampleOutput = seo?.sampleOutput ?? p.example?.output ?? "";
  const relatedGuides = Array.from(
    new Map(
      [
        ...(seo?.relatedGuides ?? []),
        ...guides
          .filter((guide) => guide.parserHrefs.some((href) => href === `/parsers/${canonicalSlug}`))
          .map((guide) => ({ href: guide.href, label: guide.title })),
      ].map((guide) => [guide.href, guide]),
    ).values(),
  );

  const structuredData = [
    {
      "@context": "https://schema.org",
      "@type": "BreadcrumbList",
      itemListElement: [
        { "@type": "ListItem", position: 1, name: "Parsers", item: `${site.origin}/parsers` },
        {
          "@type": "ListItem",
          position: 2,
          name: seo?.title ?? genericParserTitle(p),
          item: `${site.origin}/parsers/${canonicalSlug}`,
        },
      ],
    },
    ...(seo
      ? [
          {
          "@context": "https://schema.org",
          "@type": "WebApplication",
          name: seo.title,
          description: seo.description,
          url: `${site.origin}/parsers/${canonicalSlug}`,
          applicationCategory: "DeveloperApplication",
          operatingSystem: "Any",
          browserRequirements: "A browser with WebAssembly support",
          isAccessibleForFree: true,
          featureList: seo.outputDetails.map(stripInlineCode),
          author: { "@type": "Person", name: site.author, url: site.authorUrl },
          },
        ]
      : []),
  ];

  return (
    <div className="mx-auto max-w-5xl px-5 py-14">
      <script
        type="application/ld+json"
        dangerouslySetInnerHTML={{
          __html: JSON.stringify(structuredData).replaceAll("<", "\\u003c"),
        }}
      />
      <nav className="font-mono text-xs text-[var(--color-muted)]" aria-label="Breadcrumb">
        <Link href="/parsers" className="hover:text-[var(--color-ink)]">
          parsers
        </Link>
        <span className="text-[var(--color-faint)]"> / </span>
        <span>{canonicalSlug}</span>
      </nav>

      {seo ? (
        <div className="mt-4">
          <p className="font-mono text-xs tracking-wide text-[var(--color-muted)] uppercase">
            {seo.eyebrow}
          </p>
          <h1 className="mt-3 max-w-4xl text-[clamp(2.4rem,6vw,4.2rem)]">{seo.title}</h1>
          <p className="mt-4 max-w-3xl text-lg leading-8 text-[var(--color-muted)]">
            {seo.intro}
          </p>
          <p className="mt-4 flex flex-wrap items-center gap-x-3 gap-y-1 font-mono text-xs text-[var(--color-faint)]">
            <span>{p.name}</span>
            <span className="text-[var(--color-key)]">{p.argument}</span>
            <span>v{p.version}</span>
          </p>
        </div>
      ) : (
        <>
          <div className="mt-4 flex flex-wrap items-baseline gap-x-4 gap-y-2">
            <h1 className="font-mono text-4xl font-semibold tracking-tight">{canonicalSlug}</h1>
            <code className="font-mono text-sm text-[var(--color-key)]">{p.argument}</code>
            <span className="font-mono text-xs text-[var(--color-faint)]">v{p.version}</span>
          </div>
          <p className="mt-3 max-w-2xl text-lg text-[var(--color-muted)]">{p.description}</p>
        </>
      )}

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

      {seo && (
        <>
          <section className="mt-10" aria-labelledby="try-parser">
            <div className="mb-5 flex flex-wrap items-end justify-between gap-3">
              <div>
                <h2 id="try-parser" className="text-2xl">
                  Try the real parser
                </h2>
                <p className="mt-2 max-w-2xl text-sm text-[var(--color-muted)]">
                  Paste input on the left. The same Rust parser that ships in the binary runs
                  locally in this page and writes its JSON on the right.
                </p>
              </div>
              <span className="font-mono text-[11px] text-[var(--color-str)]">browser · WebAssembly</span>
            </div>
            <ParserWorkbench
              parser={p.name}
              argument={p.argument}
              sampleInput={sampleInput}
              sampleOutput={sampleOutput}
              accept={acceptedFiles(p.name)}
            />
          </section>

          <aside className="mt-8 border-l-2 border-[var(--color-num)] pl-5">
            <p className="font-mono text-[11px] tracking-wide text-[var(--color-num)] uppercase">
              Read before relying on the result
            </p>
            <p className="mt-2 max-w-3xl text-sm leading-6 text-[var(--color-muted)]">
              {renderInlineCode(seo.privacyAndValidation)}
            </p>
          </aside>

          <section className="mt-12 grid gap-8 border-y py-10 md:grid-cols-2">
            <div>
              <h2 className="text-xl">Accepted input</h2>
              <p className="mt-3 text-sm leading-6 text-[var(--color-muted)]">
                {renderInlineCode(seo.acceptedInput)}
              </p>
            </div>
            <div>
              <h2 className="text-xl">What the JSON contains</h2>
              <ul className="mt-3 space-y-3 text-sm leading-6 text-[var(--color-muted)]">
                {seo.outputDetails.map((detail) => (
                  <li key={detail} className="grid grid-cols-[0.65rem_minmax(0,1fr)] gap-2">
                    <span className="pt-px text-[var(--color-key)]" aria-hidden="true">·</span>
                    <span>{renderInlineCode(detail)}</span>
                  </li>
                ))}
              </ul>
            </div>
          </section>

          <section className="mt-10" aria-labelledby="parser-tasks">
            <h2 id="parser-tasks" className="text-xl">What you can do here</h2>
            <ul className="mt-4 grid gap-2 sm:grid-cols-2">
              {seo.queryLanguage.map((task) => (
                <li key={task} className="border-l pl-3 text-sm leading-6 text-[var(--color-muted)]">
                  {task}
                </li>
              ))}
            </ul>
          </section>
        </>
      )}

      <section className={seo ? "mt-12" : "mt-8 max-w-2xl"}>
        {seo && <h2 className="text-2xl">Use the same parser in a shell</h2>}
        <div className={seo ? "mt-4 max-w-2xl" : undefined}>
          <CopyLine command={pipeline} />
        </div>
        {seo && (
          <p className="mt-3 max-w-2xl text-sm text-[var(--color-muted)]">
            The browser tool is for inspection. The CLI form is the useful one in scripts,
            pipes, containers, and repeatable checks.
          </p>
        )}
      </section>

      {p.name === "x509_cert" && (
        <section className="mt-12 border-y py-10">
          <h2 className="text-2xl">Inspect PEM and DER certificates with OpenSSL</h2>
          <p className="mt-3 max-w-3xl text-[var(--color-muted)]">
            OpenSSL is the direct choice for a human-readable certificate dump; use jc-rs when the
            next command needs structured JSON. For binary DER, the first DER command inspects the
            file directly and the second converts it to PEM for jc-rs. None of these commands
            verifies trust in the certificate.
          </p>
          <div className="mt-5 grid gap-3">
            <CopyLine command="openssl x509 -in certificate.pem -noout -text" />
            <CopyLine command="openssl x509 -in certificate.pem -noout -subject -issuer -dates -serial" />
            <CopyLine command="openssl x509 -inform DER -in certificate.der -noout -text" />
            <CopyLine command="openssl x509 -inform DER -in certificate.der -outform PEM | jc-rs --x509-cert" />
          </div>
          <p className="mt-3 text-sm text-[var(--color-faint)]">
            Options and output fields are documented in the official{" "}
            <a
              href="https://docs.openssl.org/master/man1/openssl-x509/"
              className="text-[var(--color-key)] underline-offset-4 hover:underline"
            >
              openssl-x509 manual
            </a>
            .
          </p>
        </section>
      )}

      {!seo && p.example ? (
        <section className="mt-12">
          <h2 className="text-2xl">A real pair</h2>
          <p className="mt-2 max-w-2xl text-sm text-[var(--color-muted)]">
            From the reference corpus (
            <code className="font-mono">{p.example.fixture}</code>). The left pane is the input
            the fixture ships; the right pane is the corpus&apos;s expected JSON. {p.coverage ? (
              <>CI compares jc-rs with it field by field under the published differential. </>
            ) : (
              <>The coverage note below explains why this pair does not enter the measured comparison. </>
            )}
            Hover a value to see where it came from.
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
                  <span>expected output</span>
                  <span className="normal-case">json</span>
                </>
              }
            />
          </div>
        </section>
      ) : !seo ? (
        <section className="mt-12 rounded-xl border bg-[var(--color-surface)] p-5">
          <h2 className="text-lg">No worked example here</h2>
          <p className="mt-2 max-w-2xl text-sm text-[var(--color-muted)]">
            The reference corpus ships no fixture pair for this parser small enough to read
            on a page, so rather than invent one this page shows none. Run it against your own
            output.
          </p>
        </section>
      ) : null}

      {relatedGuides.length > 0 && (
        <section className="mt-12">
          <h2 className="text-2xl">
            {seo ? "Continue with the parsed JSON" : "Guides using this parser"}
          </h2>
          <div className="mt-4 grid gap-3 sm:grid-cols-2">
            {relatedGuides.map((guide) => (
              <Link
                key={guide.href}
                href={guide.href}
                className="rounded-lg border bg-[var(--color-surface)] px-4 py-3 text-sm text-[var(--color-muted)] transition-colors hover:border-[var(--color-key)] hover:text-[var(--color-ink)]"
              >
                {guide.label} <span className="text-[var(--color-key)]">→</span>
              </Link>
            ))}
          </div>
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
                oracle-valid pairs match under the published structural JSON comparison
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
              <code className="font-mono text-sm">
                jc-rs {p.magic[0].startsWith("/proc") ? procShellPath(p.magic[0]) : p.magic[0]}
              </code>
              {p.magic[0].startsWith("/proc") ? (
                <span className="mt-1 block text-sm text-[var(--color-muted)]">
                  jc-rs reads this kernel-interface file directly and dispatches it through the
                  matching /proc parser.
                </span>
              ) : (
                <span className="mt-1 block text-sm text-[var(--color-muted)]">
                  jc-rs runs the command itself and parses what it prints. Recognised for:{" "}
                  {p.magic.map((m) => `\`${m}\``).join(", ")}
                </span>
              )}
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
        jc-rs targets the schemas defined by the original Python tool. Fixture coverage above is
        the measured evidence for this parser; test the inputs your pipeline depends on.{" "}
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

function renderInlineCode(text: string): React.ReactNode[] {
  return text.split(/(`[^`]+`)/).map((part, index) =>
    part.startsWith("`") && part.endsWith("`") ? (
      <code key={index} className="font-mono text-[0.92em] text-[var(--color-ink)]">
        {part.slice(1, -1)}
      </code>
    ) : (
      part
    ),
  );
}

function stripInlineCode(text: string): string {
  return text.replaceAll("`", "");
}

function acceptedFiles(parser: string): string {
  const extensions: Record<string, string> = {
    jwt: ".jwt,.txt,text/plain",
    x509_cert:
      ".pem,.crt,.cer,.der,.txt,application/pkix-cert,application/x-x509-ca-cert,text/plain",
    x509_csr: ".pem,.csr,.txt,text/plain",
    xml: ".xml,text/xml,application/xml,text/plain",
    csv: ".csv,text/csv,text/plain",
    yaml: ".yaml,.yml,text/yaml,text/plain",
    url: ".txt,text/plain",
    toml: ".toml,text/plain",
    tsv: ".tsv,text/tab-separated-values,text/plain",
    plist: ".plist,.xml,text/xml,text/plain",
    asciitable: ".txt,.md,text/plain,text/markdown",
    asciitable_m: ".txt,.md,text/plain,text/markdown",
  };
  return extensions[parser] ?? "text/plain";
}

function seoShellInput(parser: string): string {
  const inputs: Record<string, string> = {
    jwt: `printf '%s\\n' "$JWT"`,
    x509_cert: "cat certificate.pem",
    x509_csr: "cat request.csr",
    xml: "cat document.xml",
    csv: "cat data.csv",
    yaml: "cat config.yaml",
    url: `printf '%s\\n' 'https://example.com/report.json?tag=rust'`,
    toml: "cat config.toml",
    tsv: "cat data.tsv",
    plist: "cat settings.plist",
    asciitable: "cat table.txt",
    asciitable_m: "cat multiline-table.txt",
  };
  return inputs[parser] ?? "cat input.txt";
}

function procShellPath(path: string): string {
  return path.replaceAll("<pid>", "$PID").replaceAll("<fd>", "$FD");
}

function genericParserTitle(parser: Parser): string {
  return `${slugOf(parser)} output to JSON`;
}

function genericParserDescription(parser: Parser): string {
  const summaryText = parser.description.replaceAll("`", "").replace(/[.\s]+$/, "");
  const action = `Use jc-rs ${parser.argument} to produce structured JSON.`;
  const combined = `${summaryText}. ${action}`;
  return combined.length <= 160 ? combined : `${summaryText}.`;
}
