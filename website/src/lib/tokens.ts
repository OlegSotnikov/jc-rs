export type JsonToken = {
  text: string;
  kind: "key" | "string" | "number" | "punct" | "plain";
  /** The bare literal a reader could look for in the raw command output. */
  literal?: string;
};

const JSON_RE = /("(?:\\.|[^"\\])*")(\s*:)?|(-?\d+(?:\.\d+)?(?:[eE][+-]?\d+)?)|(true|false|null)/g;

/**
 * Split pretty-printed JSON into coloured tokens.
 *
 * Deliberately a lexer over the printed text rather than a walk of the parsed
 * value: the point is to keep the exact bytes the parser emitted, including the
 * indentation, so what the reader sees is what jc-rs actually wrote.
 */
export function tokenizeJson(source: string): JsonToken[] {
  const out: JsonToken[] = [];
  let last = 0;
  for (const m of source.matchAll(JSON_RE)) {
    const at = m.index;
    if (at > last) out.push({ text: source.slice(last, at), kind: "punct" });

    if (m[1] !== undefined && m[2] !== undefined) {
      out.push({ text: m[1], kind: "key", literal: JSON.parse(m[1]) as string });
      out.push({ text: m[2], kind: "punct" });
    } else if (m[1] !== undefined) {
      out.push({ text: m[1], kind: "string", literal: JSON.parse(m[1]) as string });
    } else if (m[3] !== undefined) {
      out.push({ text: m[3], kind: "number", literal: m[3] });
    } else {
      out.push({ text: m[4], kind: "number", literal: m[4] });
    }
    last = at + m[0].length;
  }
  if (last < source.length) out.push({ text: source.slice(last), kind: "punct" });
  return out;
}

/**
 * Split raw command output into whitespace-separated runs, keeping the gaps so
 * the column layout survives. Each run is a candidate match for a JSON value.
 */
export function tokenizeRaw(source: string): { text: string; word: boolean }[] {
  const out: { text: string; word: boolean }[] = [];
  const re = /(\s+)/g;
  let last = 0;
  for (const m of source.matchAll(re)) {
    if (m.index > last) out.push({ text: source.slice(last, m.index), word: true });
    out.push({ text: m[0], word: false });
    last = m.index + m[0].length;
  }
  if (last < source.length) out.push({ text: source.slice(last), word: true });
  return out;
}

/**
 * Does this raw-output run correspond to a JSON value?
 *
 * Exact match first, then the two rewrites jc's schemas apply most often: a
 * unit suffix stripped off a number (`1.9G` -> `1.9`), and a percentage
 * (`0%` -> `0`). Anything looser starts lighting up unrelated columns.
 */
export function matchesLiteral(word: string, literal: string | null): boolean {
  if (!literal || !word) return false;
  if (word === literal) return true;
  const stripped = word.replace(/[%,]$/, "");
  if (stripped === literal) return true;
  const num = word.match(/^(-?\d+(?:\.\d+)?)[KMGTPE]i?B?$/);
  return num?.[1] === literal;
}
