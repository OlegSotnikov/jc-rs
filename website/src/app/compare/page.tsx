import type { Metadata } from "next";
import Link from "next/link";
import { BenchChart } from "@/components/BenchChart";
import { benchmarks, site } from "@/lib/site";
import { summary } from "@/lib/parsers";

export const metadata: Metadata = {
  title: "jc-rs compared with jc",
  description:
    "jc-rs is a single static binary with no runtime to install. jc is the original Python tool. They emit the same JSON, so switching costs one word in a pipeline.",
  alternates: { canonical: "/compare" },
};

const startup = benchmarks[0];

const ROWS: { label: string; rs: string; jc: string; note?: string }[] = [
  {
    label: "What you install",
    rs: "One static binary, 2.3 MB",
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
    note: "Per invocation. In a loop over 200 hosts that is 26 seconds of interpreter startup.",
  },
  {
    label: "Parsers",
    rs: `${summary.documented}`,
    jc: "~200",
  },
  {
    label: "Output",
    rs: "Identical JSON",
    jc: "Identical JSON",
    note: "Same schemas, same field names, same types. Anything downstream keeps working.",
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
      <h1 className="mt-3 text-4xl">jc-rs and jc</h1>
      <p className="mt-4 max-w-2xl text-lg text-[var(--color-muted)]">
        jc is the original tool that decided this category: a Python program that turns command
        output into JSON. jc-rs does the same job as a compiled binary, and emits the same JSON,
        so moving between them costs one word in a pipeline.
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
        <h2 className="text-2xl">Where the time goes</h2>
        <p className="mt-3 max-w-2xl text-[var(--color-muted)]">
          Startup is where the gap is widest, and startup is what a loop pays. Over 200 hosts, jc
          spends 21 seconds inside the Python interpreter before parsing a byte; jc-rs spends one.
          On bulk throughput the lead settles at 3× to 6×, where both are bound by the same
          per-field work.
        </p>
        <BenchChart />
      </section>

      <section className="mt-12">
        <h2 className="text-2xl">Switching</h2>
        <p className="mt-3 max-w-2xl text-[var(--color-muted)]">
          The output is the same, so the change is the command name. Nothing downstream of the
          pipe needs touching.
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
          {summary.tested} oracle-valid pairs from the full reference corpus match byte for byte,
          and the run fails CI below 100%. What the number excludes is published next to it.
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
        set and the one most workloads actually pay.
      </p>
    </div>
  );
}
