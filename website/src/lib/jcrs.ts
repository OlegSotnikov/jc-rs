"use client";

/**
 * Lazy handle on the WebAssembly build of jc-rs.
 *
 * The module is ~1.3 MB over the wire, so nothing loads it on first paint. The
 * hero renders a fixture pair the server already knows; this upgrades it to a
 * live parser once the reader shows interest (focus, typing, or browser idle).
 */

export type JcRs = {
  parse: (name: string, input: string) => unknown;
  parseRaw: (name: string, input: string) => unknown;
  parsers: () => string[];
  version: () => string;
};

let pending: Promise<JcRs> | null = null;

export function loadJcRs(): Promise<JcRs> {
  if (!pending) {
    pending = (async () => {
      // Kept out of the bundle on purpose: the file is served from public/ so
      // that a visitor who never opens the converter never downloads it.
      const mod = await import(
        /* webpackIgnore: true */ /* turbopackIgnore: true */ "/wasm/jc-rs.js"
      );
      await mod.default("/wasm/jc-rs_bg.wasm");
      return mod as JcRs;
    })().catch((err) => {
      // Let the next attempt retry rather than caching a transport failure as
      // a permanent "no parser here".
      pending = null;
      throw err;
    });
  }
  return pending;
}

export type ParseResult =
  | { ok: true; json: string; micros: number }
  | { ok: false; error: string };

export function runParse(mod: JcRs, parser: string, input: string): ParseResult {
  const started = performance.now();
  try {
    const value = mod.parse(parser, input);
    const micros = Math.round((performance.now() - started) * 1000);
    return { ok: true, json: JSON.stringify(value, null, 2), micros };
  } catch (err) {
    return { ok: false, error: err instanceof Error ? err.message : String(err) };
  }
}
