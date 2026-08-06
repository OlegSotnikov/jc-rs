"use client";

import { useMemo, useRef, useState } from "react";
import { matchesLiteral, tokenizeJson, tokenizeRaw } from "@/lib/tokens";

/**
 * The two panes, and the link between them.
 *
 * Hovering a value in the JSON lights the run of raw output it was read from.
 * The correspondence is computed from the value itself, so it works for every
 * parser rather than for one hand-annotated example.
 */
export function Panes({
  input,
  output,
  inputLabel,
  outputLabel,
  error,
  onInputChange,
  busy,
}: {
  input: string;
  output: string;
  inputLabel: React.ReactNode;
  outputLabel: React.ReactNode;
  error?: string | null;
  onInputChange?: (value: string) => void;
  busy?: boolean;
}) {
  const [lit, setLit] = useState<string | null>(null);
  const highlight = useRef<HTMLPreElement>(null);

  const rawTokens = useMemo(() => tokenizeRaw(input), [input]);
  const jsonTokens = useMemo(() => tokenizeJson(output), [output]);

  const editable = Boolean(onInputChange);

  return (
    <div className="grid gap-px overflow-hidden rounded-xl border bg-[var(--color-rule)] md:grid-cols-2">
      <section className="flex min-w-0 flex-col bg-[var(--color-surface)]">
        <PaneBar>{inputLabel}</PaneBar>

        {/* Editable: the textarea is the only thing that scrolls, and the
            highlighted layer under it is moved to match. Letting both scroll
            gives the pane two scrollbars that drift apart as you type. */}
        <div
          className={`relative max-h-[26rem] min-h-[12rem] flex-1 ${
            editable
              ? "overflow-hidden focus-within:ring-1 focus-within:ring-[var(--color-key)] focus-within:ring-inset"
              : "overflow-auto"
          }`}
        >
          <pre
            ref={highlight}
            aria-hidden={editable}
            className={`px-4 py-3 font-mono text-[12.5px] leading-[1.65] whitespace-pre ${
              editable ? "pointer-events-none absolute inset-0 overflow-hidden" : ""
            }`}
          >
            {rawTokens.map((t, i) =>
              t.word ? (
                <span key={i} className="tok" data-lit={matchesLiteral(t.text, lit)}>
                  {t.text}
                </span>
              ) : (
                <span key={i}>{t.text}</span>
              ),
            )}
          </pre>
          {editable && (
            <textarea
              value={input}
              onChange={(e) => onInputChange?.(e.target.value)}
              onScroll={(e) => {
                const el = e.currentTarget;
                if (highlight.current) {
                  highlight.current.scrollLeft = el.scrollLeft;
                  highlight.current.scrollTop = el.scrollTop;
                }
              }}
              spellCheck={false}
              wrap="off"
              aria-label="Command output to convert"
              className="absolute inset-0 h-full w-full resize-none overflow-auto bg-transparent px-4 py-3 font-mono text-[12.5px] leading-[1.65] whitespace-pre text-transparent caret-[var(--color-ink)] outline-none focus-visible:outline-none"
            />
          )}
        </div>
      </section>

      <section className="flex min-w-0 flex-col bg-[var(--color-surface)]">
        <PaneBar>{outputLabel}</PaneBar>
        <div className="max-h-[26rem] min-h-[12rem] flex-1 overflow-auto">
          {error ? (
            <p className="px-4 py-3 font-mono text-[12.5px] leading-[1.65] text-[var(--color-num)]">
              {error}
            </p>
          ) : (
            <pre
              data-busy={busy}
              className="px-4 py-3 font-mono text-[12.5px] leading-[1.65] whitespace-pre transition-opacity data-[busy=true]:opacity-45"
            >
              {jsonTokens.map((t, i) => {
                if (t.kind === "punct" || t.kind === "plain") {
                  return <span key={i} className="jpunct">{t.text}</span>;
                }
                if (t.kind === "key") {
                  return <span key={i} className="jkey">{t.text}</span>;
                }
                const cls = t.kind === "string" ? "jstr" : "jnum";
                return (
                  <span
                    key={i}
                    className={cls}
                    onMouseEnter={() => setLit(t.literal ?? null)}
                    onMouseLeave={() => setLit(null)}
                    data-lit={lit !== null && lit === t.literal}
                  >
                    {t.text}
                  </span>
                );
              })}
            </pre>
          )}
        </div>
      </section>
    </div>
  );
}

function PaneBar({ children }: { children: React.ReactNode }) {
  return (
    <div className="flex items-center justify-between gap-3 border-b bg-[var(--color-sunk)] px-4 py-2 font-mono text-[11px] tracking-wide text-[var(--color-muted)] uppercase">
      {children}
    </div>
  );
}
