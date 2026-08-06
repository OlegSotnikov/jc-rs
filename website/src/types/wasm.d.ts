/**
 * The wasm-pack bundle is served from `public/` rather than imported through
 * the bundler, so TypeScript needs to be told what lives behind the URL. A
 * wildcard pattern is required: an ambient module name starting with `/` is
 * read as a path and never matches.
 * Mirrors crates/jc-rs-wasm/pkg/jc-rs.d.ts.
 */
declare module "*/jc-rs.js" {
  export default function init(module_or_path?: string): Promise<unknown>;
  export function parse(name: string, input: string): unknown;
  export function parseRaw(name: string, input: string): unknown;
  export function parserInfo(name: string): unknown;
  export function parsers(): string[];
  export function version(): string;
}
