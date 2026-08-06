"use client";

import { useMemo, useState } from "react";

type Item = {
  name: string;
  slug: string;
  argument: string;
  description: string;
  group: string;
};

export function ParserSearch({ items }: { items: Item[] }) {
  const [q, setQ] = useState("");
  const query = q.trim().toLowerCase();

  const hits = useMemo(() => {
    if (query.length < 1) return [];
    return items
      .filter(
        (i) =>
          i.name.includes(query) ||
          i.argument.includes(query) ||
          i.description.toLowerCase().includes(query),
      )
      .slice(0, 24);
  }, [items, query]);

  return (
    <div className="relative">
      <input
        type="search"
        value={q}
        onChange={(e) => setQ(e.target.value)}
        placeholder={`Search ${items.length} parsers: ps, dig, /etc/hosts, certificate…`}
        aria-label="Search parsers"
        className="w-full rounded-lg border bg-[var(--color-surface)] px-4 py-3 font-mono text-sm outline-none transition-colors focus:border-[var(--color-key)]"
      />

      {query.length > 0 && (
        <div className="absolute z-30 mt-2 max-h-96 w-full overflow-y-auto rounded-lg border bg-[var(--color-surface)] shadow-lg">
          {hits.length === 0 ? (
            <p className="px-4 py-3 text-sm text-[var(--color-muted)]">
              No parser matches “{q}”. jc-rs implements jc&apos;s set, so if jc has no parser
              for it, neither does this.
            </p>
          ) : (
            <ul>
              {hits.map((h) => (
                <li key={h.slug}>
                  <a
                    href={`/parsers/${h.slug}`}
                    className="flex items-baseline gap-3 border-b px-4 py-2.5 last:border-b-0 hover:bg-[var(--color-sunk)]"
                  >
                    <code className="font-mono text-sm">{h.name}</code>
                    <span className="truncate text-xs text-[var(--color-muted)]">
                      {h.description}
                    </span>
                    <span className="ml-auto shrink-0 font-mono text-[10px] text-[var(--color-faint)]">
                      {h.group}
                    </span>
                  </a>
                </li>
              ))}
            </ul>
          )}
        </div>
      )}
    </div>
  );
}
