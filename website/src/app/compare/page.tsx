import type { Metadata } from "next";
import Link from "next/link";
import { BenchChart } from "@/components/BenchChart";
import { benchmarks, site } from "@/lib/site";
import { summary } from "@/lib/parsers";

const description =
  "Compare jc-rs with the original Python jc, and see where jq fits: jc-rs and jc create structured JSON; jq filters and transforms it.";

export const metadata: Metadata = {
  title: "jc-rs vs jc vs jq",
  description,
  alternates: { canonical: "/compare" },
  openGraph: {
    siteName: site.name,
    type: "website",
    title: "jc-rs vs jc vs jq",
    description,
    url: `${site.origin}/compare`,
    images: [site.socialImage],
  },
  twitter: {
    card: "summary_large_image",
    title: "jc-rs vs jc vs jq",
    description,
    images: [site.socialImage],
  },
};

const startup = benchmarks[0];

const ROWS: { label: string; rs: string; jc: string; note?: string }[] = [
  {
    label: "What you install",
    rs: "One static binary",
    jc: "A Python package and its interpreter",
  },
  {
    label: "Runtime dependency",
    rs: "None",
    jc: "Python 3",
    note: "This is what decides whether it can go in a scratch container or on an appliance.",
  },
  {
    label: "Cold start",
    rs: `${startup.rs} ms`,
    jc: `${startup.jc} ms`,
    note: `Whole-process wall time per invocation. Repeated 200 times, that is ${(startup.rs * 200 / 1000).toFixed(1)} s versus ${(startup.jc * 200 / 1000).toFixed(1)} s in this benchmark.`,
  },
  {
    label: "Parsers",
    rs: `${summary.documented}`,
    jc: "~200",
  },
  {
    label: "Output",
    rs: "jc-compatible schemas",
    jc: "Reference schemas",
    note: `${summary.matched}/${summary.tested} oracle-valid fixture pairs have no structural or value differences under the published JSON comparison. Test the inputs your pipeline depends on.`,
  },
  {
    label: "Streaming",
    rs: "NDJSON as input arrives",
    jc: "NDJSON as input arrives",
  },
  {
    label: "In a browser",
    rs: "Yes, via WebAssembly",
    jc: "No",
    note: "The converter on the front page of this site is the parser set itself, running locally.",
  },
  {
    label: "As a library",
    rs: "Rust crate, and an npm package",
    jc: "Python module",
  },
  {
    label: "Distribution",
    rs: "Binaries, crates.io, Homebrew, npm, Docker",
    jc: "pip",
  },
];

export default function Compare() {
  return (
    <div className="mx-auto max-w-5xl px-5 py-14">
      <p className="font-mono text-xs tracking-wide text-[var(--color-muted)] uppercase">
        Comparison
      </p>
      <h1 className="mt-3 text-4xl">jc-rs, jc, and jq</h1>
      <p className="mt-4 max-w-2xl text-lg text-[var(--color-muted)]">
        jc is the original tool that decided this category: a Python program that turns command
        output into JSON. jc-rs implements those schemas in a compiled binary and measures its
        output against the shared fixture corpus. jq starts after that conversion: it reads JSON
        and selects, reshapes, or aggregates it.
      </p>

      <div className="mt-8 grid gap-3 sm:grid-cols-2">
        <div className="rounded-xl border bg-[var(--color-surface)] p-5">
          <p className="font-mono text-sm font-semibold">
            jc<span className="text-[var(--color-key)]">-rs</span>
          </p>
          <p className="mt-2 text-sm text-[var(--color-muted)]">
            Rust. One static binary, nothing to install alongside it. Runs where a language
            runtime cannot go: scratch containers, embedded images, a browser tab.
          </p>
        </div>
        <div className="rounded-xl border bg-[var(--color-surface)] p-5">
          <p className="font-mono text-sm font-semibold">jc</p>
          <p className="mt-2 text-sm text-[var(--color-muted)]">
            Python, by Kelly Brazil. The original, and the tool whose JSON schemas both
            implementations produce.{" "}
            <a href={site.jc} className="text-[var(--color-key)] underline-offset-4 hover:underline">
              github.com/kellyjonbrazil/jc
            </a>
          </p>
        </div>
      </div>

      <section className="mt-12">
        <h2 className="text-2xl">Where jq fits</h2>
        <p className="mt-3 max-w-2xl text-[var(--color-muted)]">
          jq does not know where the columns in <code className="font-mono">ss</code> output end,
          and jc-rs does not provide jq&apos;s query language. Put them next to each other: the
          parser establishes the records, then jq asks a question of those records.
        </p>
        <div className="mt-5 overflow-hidden rounded-xl border bg-[var(--color-surface)]">
          <div className="grid gap-px bg-[var(--color-rule)] sm:grid-cols-[1fr_auto_1fr_auto_1fr]">
            {[
              ["raw output", "ss -tlnp"],
              ["parse", "jc-rs --ss"],
              ["query JSON", "jq '[.[].local_port] | unique'"],
            ].map(([label, command], index) => (
              <div key={label} className="contents">
                {index > 0 && (
                  <span className="hidden items-center bg-[var(--color-paper)] px-3 font-mono text-[var(--color-faint)] sm:flex">
                    |
                  </span>
                )}
                <div className="bg-[var(--color-surface)] p-4">
                  <p className="font-mono text-[10px] tracking-wide text-[var(--color-faint)] uppercase">
                    {label}
                  </p>
                  <code className="mt-2 block overflow-x-auto font-mono text-xs text-[var(--color-ink)]">
                    {command}
                  </code>
                </div>
              </div>
            ))}
          </div>
        </div>
        <p className="mt-3 text-sm text-[var(--color-muted)]">
          For quoting, exit handling, and complete Bash examples, read{" "}
          <Link href="/guides/bash-jc-rs-jq" className="text-[var(--color-key)] underline-offset-4 hover:underline">
            Bash, jc-rs, and jq
          </Link>
          .
        </p>
      </section>

      <section className="mt-12">
        <h2 className="text-2xl">Side by side</h2>
        <div className="mt-5 overflow-hidden rounded-xl border bg-[var(--color-surface)]">
          <div className="grid grid-cols-[minmax(0,1fr)_minmax(0,1fr)_minmax(0,1fr)] gap-4 border-b bg-[var(--color-sunk)] px-5 py-2.5 font-mono text-[11px] tracking-wide text-[var(--color-faint)] uppercase">
            <span />
            <span className="text-[var(--color-key)]">jc-rs</span>
            <span>jc</span>
          </div>
          {ROWS.map((r) => (
            <div key={r.label} className="border-b px-5 py-4 last:border-b-0">
              <div className="grid grid-cols-[minmax(0,1fr)_minmax(0,1fr)_minmax(0,1fr)] gap-4">
                <span className="text-sm text-[var(--color-muted)]">{r.label}</span>
                <span className="font-mono text-sm">{r.rs}</span>
                <span className="font-mono text-sm text-[var(--color-muted)]">{r.jc}</span>
              </div>
              {r.note && (
                <p className="mt-2 max-w-2xl text-xs text-[var(--color-faint)]">{r.note}</p>
              )}
            </div>
          ))}
        </div>
      </section>

      <section className="mt-12">
        <h2 className="text-2xl">Measured process time</h2>
        <p className="mt-3 max-w-2xl text-[var(--color-muted)]">
          The harness measures complete process wall time, once per invocation. Repeating the cold
          start row 200 times totals {(startup.jc * 200 / 1000).toFixed(1)} seconds for jc and{" "}
          {(startup.rs * 200 / 1000).toFixed(1)} seconds for jc-rs on the measured host. The larger
          fixture rows below measure parsing and process overhead together; the benchmark does not
          assign those milliseconds to individual runtime components.
        </p>
        <BenchChart />
      </section>

      <section className="mt-12">
        <h2 className="text-2xl">Switching</h2>
        <p className="mt-3 max-w-2xl text-[var(--color-muted)]">
          For a parser and input that match the compatibility contract, the command name can be
          the only change. Compare representative output before moving a production pipeline.
        </p>
        <pre className="mt-4 overflow-x-auto rounded-lg border bg-[var(--color-sunk)] p-4 font-mono text-xs leading-relaxed">
          {`- ps aux | jc --ps | jq '.[0]'
+ ps aux | jc-rs --ps | jq '.[0]'`}
        </pre>
        <p className="mt-4 text-sm text-[var(--color-muted)]">
          The binary is called <code className="font-mono">jc-rs</code> precisely so both can sit
          in <code className="font-mono">PATH</code> at once. Release archives carry a{" "}
          <code className="font-mono">jc</code> alias for anyone who wants the shorter name, and
          nothing installs it by default.
        </p>
      </section>

      <section className="mt-12">
        <h2 className="text-2xl">When jc is the better pick</h2>
        <ul className="mt-4 space-y-3 text-[var(--color-muted)]">
          <li>
            You are writing Python and want the parsers as a module in the same process. jc-rs
            gives you a Rust crate and a WebAssembly package, not a Python one.
          </li>
          <li>
            You need a parser jc-rs has not implemented. The set is close but not identical, and{" "}
            <Link href="/parsers" className="text-[var(--color-key)] underline-offset-4 hover:underline">
              the index
            </Link>{" "}
            lists exactly what ships.
          </li>
          <li>
            Startup time is irrelevant to you and a runtime is already installed. One invocation
            at a prompt does not care about 110 ms.
          </li>
        </ul>
      </section>

      <section className="mt-12 rounded-xl border bg-[var(--color-surface)] p-6">
        <h2 className="text-xl">The compatibility claim, and how to check it</h2>
        <p className="mt-3 text-sm text-[var(--color-muted)]">
          &ldquo;Same JSON&rdquo; is easy to say, so jc-rs measures it: {summary.matched} of{" "}
          {summary.tested} oracle-valid pairs drawn from the full corpus have no structural or
          value differences under the published comparison, and the run fails CI below 100%.
          What the number excludes is published next to it.
        </p>
        <Link
          href="/compatibility"
          className="mt-4 inline-block rounded-md border px-4 py-2 text-sm transition-colors hover:border-[var(--color-key)]"
        >
          How the number is produced
        </Link>
      </section>

      <p className="mt-10 text-sm text-[var(--color-faint)]">
        Cold start figure above: {startup.jc} ms against {startup.rs} ms, the widest gap in the
        measured set. Method: one fresh process per run, fastest result retained from repeated
        runs.
      </p>
    </div>
  );
}
