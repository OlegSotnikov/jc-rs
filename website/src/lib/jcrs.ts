"use client";

import summaryData from "@/data/summary.json";

export const MAX_INTERACTIVE_INPUT_CHARACTERS = 3_000_000;

/**
 * Lazy handle on the WebAssembly build of jc-rs.
 *
 * The module is about 1.5 MB compressed, so nothing loads it on first paint. The
 * hero renders a fixture pair the server already knows; this upgrades it to a
 * live parser once the reader shows interest (focus, typing, or browser idle).
 */

export type JcRs = {
  parse: (name: string, input: string) => unknown;
  parseJson?: (name: string, input: string) => string;
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
      // The assets are cached as immutable, so the release version is part of
      // the URL. A returning visitor must not pair a new client chunk with a
      // year-cached WebAssembly API from an older release.
      const assetVersion = encodeURIComponent(summaryData.version);
      const mod = await import(
        /* webpackIgnore: true */ /* turbopackIgnore: true */ `/wasm/jc-rs.js?v=${assetVersion}`
      );
      await mod.default({ module_or_path: `/wasm/jc-rs_bg.wasm?v=${assetVersion}` });
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
  if (input.length > MAX_INTERACTIVE_INPUT_CHARACTERS) {
    return {
      ok: false,
      error:
        "Interactive input is limited to 3 million characters. Use the jc-rs CLI for larger data.",
    };
  }
  const started = performance.now();
  try {
    // parseJson is emitted by current bundles and preserves nulls and integers
    // outside JavaScript's safe range. The fallback keeps a newly deployed
    // client usable if an older immutable bundle is still in a browser cache.
    const json = mod.parseJson
      ? mod.parseJson(parser, input)
      : JSON.stringify(mod.parse(parser, input), null, 2);
    const micros = Math.round((performance.now() - started) * 1000);
    return { ok: true, json, micros };
  } catch (err) {
    return { ok: false, error: err instanceof Error ? err.message : String(err) };
  }
}
