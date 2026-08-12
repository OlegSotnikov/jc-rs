"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import { Panes } from "@/components/Panes";
import { loadJcRs, runParse, type JcRs } from "@/lib/jcrs";

export type Preset = {
  name: string;
  argument: string;
  command: string;
  input: string;
  output: string;
};

type View = {
  input: string;
  output: string;
  error: string | null;
  micros: number | null;
};

/**
 * The hero converter.
 *
 * Renders the server-known fixture pair immediately, so the first paint carries
 * real content and costs no JavaScript. The 1.5 MB compressed WebAssembly build arrives
 * afterwards, on idle or on the first keystroke, and from then on every edit is
 * parsed by the same code that ships in the binary.
 *
 * Parsing happens in the handlers rather than in an effect: the input and the
 * result change together, so there is nothing to synchronise after the fact.
 */
export function Converter({ presets }: { presets: Preset[] }) {
  const [active, setActive] = useState(0);
  const [view, setView] = useState<View>({
    input: presets[0].input,
    output: presets[0].output,
    error: null,
    micros: null,
  });
  const [live, setLive] = useState(false);
  const [failed, setFailed] = useState(false);
  const mod = useRef<JcRs | null>(null);
  const loadingRef = useRef(false);
  const activeRef = useRef(0);

  const preset = presets[active];

  /** Parse with whatever module state we have; fall back to leaving the pair alone. */
  const render = useCallback((parser: string, input: string, m: JcRs | null): View => {
    if (!m) return { input, output: "", error: null, micros: null };
    if (!input.trim()) return { input, output: "", error: null, micros: null };
    const r = runParse(m, parser, input);
    return r.ok
      ? { input, output: r.json, error: null, micros: r.micros }
      : { input, output: "", error: r.error, micros: null };
  }, []);

  const warm = useCallback(async () => {
    if (mod.current || loadingRef.current) return;
    loadingRef.current = true;
    setFailed(false);
    try {
      const m = await loadJcRs();
      mod.current = m;
      setLive(true);
      // Re-parse what is on screen so the timing line stops saying "loading".
      setView((v) => render(presets[activeRef.current].name, v.input, m));
    } catch {
      // The pre-rendered pair stays on screen; only the live upgrade is lost.
      setFailed(true);
      setLive(false);
    } finally {
      loadingRef.current = false;
    }
  }, [presets, render]);

  // Fetch the module once the browser is otherwise idle. Safari has no
  // requestIdleCallback, so fall back to a timer there.
  useEffect(() => {
    const idle = "requestIdleCallback" in window;
    const id = idle
      ? requestIdleCallback(() => void warm())
      : window.setTimeout(() => void warm(), 1800);
    return () => {
      if (idle) cancelIdleCallback(id);
      else window.clearTimeout(id);
    };
  }, [warm]);

  function choose(index: number) {
    activeRef.current = index;
    setActive(index);
    const p = presets[index];
    setView(
      mod.current
        ? render(p.name, p.input, mod.current)
        : { input: p.input, output: p.output, error: null, micros: null },
    );
  }

  function edit(next: string) {
    void warm();
    setView(
      mod.current
        ? render(preset.name, next, mod.current)
        : { input: next, output: "", error: null, micros: null },
    );
  }

  const status = view.error
    ? "no match"
    : live && view.micros !== null
      ? `${(view.micros / 1000).toFixed(view.micros < 1000 ? 2 : 1)} ms, in your browser`
      : failed
        ? "prerendered"
        : "loading WebAssembly";

  return (
    <div className="rise">
      <div className="mb-3 flex flex-wrap items-center gap-1.5">
        {presets.map((p, i) => (
          <button
            key={p.name}
            type="button"
            onClick={() => choose(i)}
            aria-pressed={i === active}
            className="rounded-md border px-2.5 py-1 font-mono text-xs text-[var(--color-muted)] transition-colors hover:border-[var(--color-key)] hover:text-[var(--color-ink)] aria-pressed:border-[var(--color-key)] aria-pressed:bg-[color-mix(in_oklab,var(--color-key)_12%,transparent)] aria-pressed:text-[var(--color-ink)]"
          >
            {p.command}
          </button>
        ))}
      </div>

      <Panes
        input={view.input}
        output={view.output}
        error={view.error}
        busy={!live && view.input !== preset.input}
        onInputChange={edit}
        inputLabel={
          <>
            <span>{preset.command}</span>
            <span className="normal-case">editable</span>
          </>
        }
        outputLabel={
          <>
            <span>jc-rs {preset.argument}</span>
            <span className="normal-case tabular-nums">{status}</span>
          </>
        }
      />

      <p className="mt-3 text-sm text-[var(--color-muted)]">
        Hover over a JSON value, or focus the JSON pane and use the Left and Right arrow keys, to
        highlight its source. Edits are parsed in this page by the same WebAssembly parser code
        that ships in the binary; jc-rs does not send pasted input to a server.
      </p>
    </div>
  );
}
