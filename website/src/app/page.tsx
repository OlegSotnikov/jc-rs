import Link from "next/link";
import type { Metadata } from "next";
import { BenchChart } from "@/components/BenchChart";
import { Converter, type Preset } from "@/components/Converter";
import { CopyLine } from "@/components/CopyLine";
import { benchmarks, install, site } from "@/lib/site";
import { featured, grouped, parsers, summary } from "@/lib/parsers";

export const metadata: Metadata = {
  alternates: { canonical: "/" },
};

const HERO_PARSERS = ["df", "free", "dig", "ss", "lsblk", "who"];

function presets(): Preset[] {
  const picked = HERO_PARSERS.map((n) => parsers.find((p) => p.name === n)).filter(
    (p) => p?.example,
  );
  const fallback = featured(6);
  const list = (picked.length >= 3 ? picked : fallback).slice(0, 6);
  return list.map((p) => ({
    name: p!.name,
    argument: p!.argument,
    command: p!.magic[0] ?? p!.name,
    input: p!.example!.input,
    output: p!.example!.output,
  }));
}

export default function Home() {
  const groups = grouped();
  const heroPresets = presets();

  return (
    <>
      <section className="mx-auto max-w-6xl px-5 pt-14 pb-16 sm:pt-20">
        <p className="font-mono text-xs tracking-wide text-[var(--color-muted)] uppercase">
          v{summary.version} · {summary.documented} parsers · one static binary
        </p>
        <h1 className="mt-4 max-w-3xl text-[clamp(2.4rem,6vw,4.1rem)]">
          Pipe anything into <span className="text-[var(--color-key)]">jq</span>.
        </h1>
        <p className="mt-5 max-w-2xl text-lg text-[var(--color-muted)]">
          jc-rs turns the output of the commands you already run into JSON. One static
          binary, nothing to install alongside it. The converter below is the real parser,
          compiled to WebAssembly and running in your browser.
        </p>

        <div className="mt-9">
          <Converter presets={heroPresets} />
        </div>
      </section>

      <Evidence />
      <Speed />
      <Install />
      <Catalogue groups={groups} />
    </>
  );
}

function Evidence() {
  const excluded = summary.oracleReject + summary.unmapped + summary.noInput;
  return (
    <section className="border-y bg-[var(--color-surface)]">
      <div className="mx-auto max-w-6xl px-5 py-16">
        <div className="grid gap-12 lg:grid-cols-[minmax(0,1fr)_minmax(0,1.1fr)]">
          <div>
            <p className="font-mono text-xs tracking-wide text-[var(--color-faint)] uppercase">
              The measurement
            </p>
            <p className="mt-4 font-display text-[clamp(3.5rem,9vw,5.5rem)] leading-none font-semibold tracking-tight">
              {summary.matchRate}%
            </p>
            <div className="ruler mt-5 max-w-sm">
              <span style={{ width: `${summary.matchRate}%` }} />
            </div>
            <p className="mt-4 max-w-md text-[var(--color-muted)]">
              {summary.matched} of {summary.tested} pairs from the full reference corpus,
              byte for byte, checked on every commit. CI fails below 100%, so the number
              cannot quietly drift.
            </p>
          </div>

          <div>
            <p className="font-mono text-xs tracking-wide text-[var(--color-faint)] uppercase">
              What the number excludes, and why it says so
            </p>
            <p className="mt-4 text-[var(--color-muted)]">
              A pair counts only when the reference implementation reproduces its own
              fixture exactly. Everything else is reported by category rather than dropped: a
              harness that silently skips what it cannot handle is how a project reports 100%
              while being blind to a third of the evidence.
            </p>
            <dl className="mt-6 divide-y border-y">
              {[
                ["Tested, and matching", summary.matched, "the reference reproduces it, so do we"],
                ["Oracle reject", summary.oracleReject, "the reference cannot reproduce its own fixture"],
                ["Unmapped", summary.unmapped, "the filename resolves to no parser"],
                ["No input", summary.noInput, "expected output ships without an input file"],
              ].map(([label, value, note]) => (
                <div key={label as string} className="flex items-baseline gap-4 py-3">
                  <dt className="flex-1 text-sm">
                    {label}
                    <span className="mt-0.5 block text-xs text-[var(--color-faint)]">{note}</span>
                  </dt>
                  <dd className="font-mono text-sm tabular-nums">{value}</dd>
                </div>
              ))}
            </dl>
            <p className="mt-4 text-sm text-[var(--color-faint)]">
              {excluded} pairs reported but not tested.{" "}
              <Link href="/compatibility" className="text-[var(--color-key)] underline-offset-4 hover:underline">
                How this is produced
              </Link>
            </p>
          </div>
        </div>
      </div>
    </section>
  );
}

function Speed() {
  const startup = benchmarks[0];
  return (
    <section className="mx-auto max-w-6xl px-5 py-16">
      <h2 className="text-3xl">Startup is where it shows</h2>
      <p className="mt-3 max-w-2xl text-[var(--color-muted)]">
        A compiled binary starts in {startup.rs} ms. The Python original spends {startup.jc} ms
        inside its interpreter before it reads a byte, and pays that on every invocation — which
        is what a loop, a git hook or a run across 200 hosts is made of. On bulk throughput both
        are bound by the same per-field work.{" "}
        <Link href="/compare" className="text-[var(--color-key)] underline-offset-4 hover:underline">
          Full comparison
        </Link>
        .
      </p>

      <BenchChart />
    </section>
  );
}

function Install() {
  return (
    <section id="install" className="border-y bg-[var(--color-surface)]">
      <div className="mx-auto max-w-6xl px-5 py-16">
        <h2 className="text-3xl">Install</h2>
        <p className="mt-3 max-w-2xl text-[var(--color-muted)]">
          Five channels, all cut from the same tag. The binary is{" "}
          <code className="font-mono">jc-rs</code>, never <code className="font-mono">jc</code>:
          release archives carry a <code className="font-mono">jc</code> alias you can enable
          deliberately, and nothing installs it by default because it would shadow the
          original in <code className="font-mono">PATH</code>.
        </p>
        <div className="mt-8 grid gap-3 sm:grid-cols-2">
          {install.map((i) => (
            <CopyLine key={i.cmd} label={i.label} command={i.cmd} />
          ))}
        </div>
        <p className="mt-6 text-sm text-[var(--color-faint)]">
          Or take a static binary from{" "}
          <a href={site.releases} className="text-[var(--color-key)] underline-offset-4 hover:underline">
            the releases
          </a>
          : five targets, with completions for bash, zsh and fish and SHA256SUMS in every
          archive.
        </p>
      </div>
    </section>
  );
}

function Catalogue({ groups }: { groups: ReturnType<typeof grouped> }) {
  return (
    <section className="mx-auto max-w-6xl px-5 py-16">
      <div className="flex flex-wrap items-end justify-between gap-4">
        <div>
          <h2 className="text-3xl">{summary.documented} parsers</h2>
          <p className="mt-3 max-w-2xl text-[var(--color-muted)]">
            Every one of them has a reference page with its platforms, its magic command and a
            real fixture pair where the corpus has one.
          </p>
        </div>
        <Link
          href="/parsers"
          className="rounded-md border px-3.5 py-2 text-sm transition-colors hover:border-[var(--color-key)]"
        >
          Browse all
        </Link>
      </div>

      <div className="mt-8 grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
        {groups.map((g) => (
          <Link
            key={g.title}
            href={`/parsers#${g.title.toLowerCase().replace(/[^a-z]+/g, "-")}`}
            className="group rounded-xl border bg-[var(--color-surface)] p-5 transition-colors hover:border-[var(--color-key)]"
          >
            <div className="flex items-baseline justify-between gap-3">
              <p className="font-display text-lg font-semibold">{g.title}</p>
              <span className="font-mono text-xs text-[var(--color-faint)] tabular-nums">
                {g.items.length}
              </span>
            </div>
            <p className="mt-1.5 text-sm text-[var(--color-muted)]">{g.blurb}</p>
            <p className="mt-3 truncate font-mono text-xs text-[var(--color-faint)] transition-colors group-hover:text-[var(--color-key)]">
              {g.items.slice(0, 5).map((p) => p.name).join("  ")}
            </p>
          </Link>
        ))}
      </div>
    </section>
  );
}
