import { benchmarks, benchmarkMeta } from "@/lib/site";

/**
 * jc against jc-rs, one grouped pair of bars per scenario.
 *
 * Horizontal because the categories are words. Grouped, since the two numbers
 * are alternatives that never sum to anything. Plain elements carry the bars, so
 * they reflow on a phone without a viewBox fighting the layout.
 *
 * Every bar carries its own value label, because at a linear scale a 5 ms bar
 * beside a 547 ms one is a few pixels wide. The scale stays linear — that ratio
 * is the result — so the label is what the value is read from.
 */
export function BenchChart() {
  const peak = Math.max(...benchmarks.map((b) => Math.max(b.jc, b.rs)));

  return (
    <figure className="mt-5">
      <div className="mb-4 flex items-center gap-5 text-xs text-[var(--color-muted)]">
        {(
          [
            ["jc", "var(--color-chart-jc)"],
            ["jc-rs", "var(--color-chart-rs)"],
          ] as const
        ).map(([name, colour]) => (
          <span key={name} className="flex items-center gap-2">
            <span
              aria-hidden
              className="inline-block h-2.5 w-2.5 rounded-[2px]"
              style={{ background: colour }}
            />
            {name}
          </span>
        ))}
        <span className="ml-auto">milliseconds per run, lower is better</span>
      </div>

      <div className="overflow-hidden rounded-xl border bg-[var(--color-surface)]">
        {benchmarks.map((b) => (
          <div
            key={b.scenario}
            className="grid grid-cols-[9rem_minmax(0,1fr)_3.5rem] items-center gap-4 border-b px-5 py-4 last:border-b-0"
          >
            <div className="min-w-0">
              <div className="truncate font-mono text-sm">{b.scenario}</div>
              <div className="truncate text-xs text-[var(--color-faint)]">{b.detail}</div>
            </div>

            <div className="flex flex-col gap-1.5">
              {(
                [
                  ["jc", b.jc, "var(--color-chart-jc)"],
                  ["jc-rs", b.rs, "var(--color-chart-rs)"],
                ] as const
              ).map(([name, ms, colour]) => (
                <div key={name} className="flex items-center gap-2">
                  <div
                    // A floor, so the fastest bar is still a mark and not nothing.
                    style={{
                      width: `max(3px, ${(ms / peak) * 100}%)`,
                      background: colour,
                    }}
                    className="h-3.5 rounded"
                    title={`${name}: ${ms} ms`}
                  />
                  <span className="shrink-0 font-mono text-xs tabular-nums text-[var(--color-muted)]">
                    {ms} ms
                  </span>
                </div>
              ))}
            </div>

            <span className="text-right font-mono text-sm tabular-nums">
              {(b.jc / Math.max(b.rs, 1)).toFixed(1)}×
            </span>
          </div>
        ))}
      </div>

      <figcaption className="mt-3 text-sm text-[var(--color-faint)]">
        One harness timing both sides, {benchmarkMeta.method}. jc {benchmarkMeta.jcVersion} on
        Python {benchmarkMeta.python}, {benchmarkMeta.platform}, {benchmarkMeta.measured}. Run it
        on your own machine with <code className="font-mono">make bench-vs-jc</code>.
      </figcaption>
    </figure>
  );
}
