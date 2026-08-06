"use client";

import { useState } from "react";

export function CopyLine({ label, command }: { label?: string; command: string }) {
  const [copied, setCopied] = useState(false);

  async function copy() {
    try {
      await navigator.clipboard.writeText(command);
      setCopied(true);
      setTimeout(() => setCopied(false), 1600);
    } catch {
      // Clipboard is blocked without a user gesture in some embeddings; the
      // command is selectable text either way, so there is nothing to recover.
    }
  }

  return (
    <div className="group flex items-center gap-3 rounded-lg border bg-[var(--color-sunk)] px-4 py-3">
      <div className="min-w-0 flex-1">
        {label && (
          <p className="font-mono text-[10.5px] tracking-wide text-[var(--color-faint)] uppercase">
            {label}
          </p>
        )}
        <code className="mt-0.5 block truncate font-mono text-sm">
          <span className="text-[var(--color-punct)] select-none">$ </span>
          {command}
        </code>
      </div>
      <button
        type="button"
        onClick={copy}
        aria-label={`Copy: ${command}`}
        className="shrink-0 rounded-md border bg-[var(--color-surface)] px-2 py-1 font-mono text-[11px] text-[var(--color-muted)] transition-colors hover:border-[var(--color-key)] hover:text-[var(--color-ink)]"
      >
        {copied ? "copied" : "copy"}
      </button>
    </div>
  );
}
