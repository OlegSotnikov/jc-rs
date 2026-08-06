import type { Metadata } from "next";
import { parsers, summary } from "@/lib/parsers";
import { site } from "@/lib/site";

export const metadata: Metadata = {
  title: "How the compatibility number is produced",
  description: `jc-rs measures ${summary.matchRate}% against the full reference corpus. This is the rule that decides what enters the denominator, and what the number deliberately excludes.`,
  alternates: { canonical: "/compatibility" },
};

export default function Compatibility() {
  const covered = parsers.filter((p) => p.coverage).sort((a, b) => b.coverage!.tested - a.coverage!.tested);
  const uncovered = parsers.filter((p) => !p.coverage);

  return (
    <div className="mx-auto max-w-4xl px-5 py-14">
      <p className="font-mono text-xs tracking-wide text-[var(--color-muted)] uppercase">
        Method
      </p>
      <h1 className="mt-3 text-4xl">How the number is produced</h1>
      <p className="mt-4 text-lg text-[var(--color-muted)]">
        The product is a compatibility number anyone can check, not speed. That premise is
        worth nothing unless the way it is measured is on the table too.
      </p>

      <section className="mt-12">
        <h2 className="text-2xl">One rule</h2>
        <blockquote className="mt-4 border-l-2 border-[var(--color-key)] pl-5 text-lg">
          A fixture pair enters the denominator only when the reference implementation
          reproduces that fixture exactly.
        </blockquote>
        <p className="mt-4 text-[var(--color-muted)]">
          <code className="font-mono text-sm">tests/differential/validate.py</code> walks every{" "}
          <code className="font-mono text-sm">.json</code> fixture in the pinned corpus, runs
          the reference against the same input, and only then compares jc-rs. If the reference
          cannot reproduce its own expected output on this machine, neither implementation is
          being tested and the pair is reported instead of counted. The reference is{" "}
          <a href={site.jc} className="text-[var(--color-key)] underline-offset-4 hover:underline">
            jc {summary.jcVersion}
          </a>
          , the original Python tool.
        </p>
      </section>

      <section className="mt-12">
        <h2 className="text-2xl">What that leaves out, in the open</h2>
        <div className="mt-5 overflow-hidden rounded-xl border bg-[var(--color-surface)]">
          {[
            {
              k: "Tested",
              v: summary.tested,
              d: "The reference reproduces the fixture, so the comparison means something.",
            },
            {
              k: "Matching",
              v: summary.matched,
              d: "jc-rs writes the same bytes. This over the line above is the number.",
            },
            {
              k: "Oracle reject",
              v: summary.oracleReject,
              d: "The reference cannot reproduce its own fixture here. Nothing to measure against.",
            },
            {
              k: "Unmapped",
              v: summary.unmapped,
              d: "The fixture filename resolves to no parser. Reducing this set will lower the headline number before it raises it, which is the trade the harness exists to make visible.",
            },
            {
              k: "No input",
              v: summary.noInput,
              d: "The corpus ships the expected output without the input that produced it.",
            },
          ].map((r) => (
            <div key={r.k} className="flex gap-5 border-b px-5 py-4 last:border-b-0">
              <span className="w-32 shrink-0 font-mono text-sm">{r.k}</span>
              <span className="w-14 shrink-0 font-mono text-sm tabular-nums">{r.v}</span>
              <span className="text-sm text-[var(--color-muted)]">{r.d}</span>
            </div>
          ))}
        </div>
      </section>

      <section className="mt-12">
        <h2 className="text-2xl">Two things that decide whether it means anything</h2>
        <div className="mt-5 space-y-5">
          <Point title="The fixtures are a verbatim mirror">
            <code className="font-mono text-sm">tests/fixtures/</code> is copied verbatim from
            the pinned reference and <code className="font-mono text-sm">make check-fixtures</code>{" "}
            fails the build if any shared file differs. Without that, a failing parser could be
            made to pass by rewriting the expected output, and the compatibility number would
            measure nothing at all.
          </Point>
          <Point title="The run is pinned to TZ=PST8PDT">
            The fixtures carry <code className="font-mono text-sm">*_epoch</code> fields
            computed in local time, and the reference&apos;s own test runner pins that zone. In
            any other timezone the oracle rejects every timestamp-bearing fixture and 146 pairs
            quietly leave the denominator, which understates coverage as surely as a silent skip
            overstates it.
          </Point>
        </div>
      </section>

      <section className="mt-12">
        <h2 className="text-2xl">Per parser</h2>
        <p className="mt-3 text-[var(--color-muted)]">
          {summary.withCoverage} parsers have oracle-valid fixtures behind them. The other{" "}
          {uncovered.length} are implemented but unproven by the corpus, and this page says so
          rather than rounding them into the total.
        </p>

        <div className="mt-5 overflow-hidden rounded-xl border bg-[var(--color-surface)]">
          <div className="grid grid-cols-[minmax(0,1fr)_5rem_5rem] gap-3 border-b bg-[var(--color-sunk)] px-5 py-2.5 font-mono text-[11px] tracking-wide text-[var(--color-faint)] uppercase">
            <span>Parser</span>
            <span className="text-right">Tested</span>
            <span className="text-right">Match</span>
          </div>
          <div className="max-h-[32rem] overflow-y-auto">
            {covered.map((p) => (
              <a
                key={p.name}
                href={`/parsers/${p.name.replaceAll("_", "-")}`}
                className="grid grid-cols-[minmax(0,1fr)_5rem_5rem] gap-3 border-b px-5 py-2 last:border-b-0 hover:bg-[var(--color-sunk)]"
              >
                <code className="truncate font-mono text-sm">{p.name}</code>
                <span className="text-right font-mono text-sm tabular-nums text-[var(--color-muted)]">
                  {p.coverage!.tested}
                </span>
                <span
                  className={`text-right font-mono text-sm tabular-nums ${
                    p.coverage!.match === p.coverage!.tested
                      ? "text-[var(--color-str)]"
                      : "text-[var(--color-num)]"
                  }`}
                >
                  {p.coverage!.match}
                </span>
              </a>
            ))}
          </div>
        </div>
      </section>

      <section className="mt-12 rounded-xl border bg-[var(--color-surface)] p-6">
        <h2 className="text-xl">Check it yourself</h2>
        <p className="mt-3 text-sm text-[var(--color-muted)]">
          None of this is a claim you have to take on trust. Clone the repository, pull the
          reference submodule and run the same harness CI runs.
        </p>
        <pre className="mt-4 overflow-x-auto rounded-lg bg-[var(--color-sunk)] p-4 font-mono text-xs leading-relaxed">
          {`git clone --recurse-submodules ${site.repo}.git
cd jc-rs
make submodule deps-py
make differential`}
        </pre>
        <p className="mt-3 text-sm text-[var(--color-faint)]">
          CI runs it with <code className="font-mono">--fail-under 100</code>, so the number
          cannot drop without the build going red.
        </p>
      </section>
    </div>
  );
}

function Point({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <div className="rounded-xl border bg-[var(--color-surface)] p-5">
      <p className="font-display text-lg font-semibold">{title}</p>
      <p className="mt-2 text-[var(--color-muted)]">{children}</p>
    </div>
  );
}
