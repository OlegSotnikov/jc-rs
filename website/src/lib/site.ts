export const site = {
  name: "jc-rs",
  origin: process.env.NEXT_PUBLIC_SITE_ORIGIN ?? "https://jc-rs.com",
  tagline: "Command output to JSON, from one static binary",
  repo: "https://github.com/OlegSotnikov/jc-rs",
  author: "Oleg Sotnikov",
  authorUrl: "https://oleg.is",
  jc: "https://github.com/kellyjonbrazil/jc",
  dockerHub: "https://hub.docker.com/r/appmasterio/jc-rs",
  cratesIo: "https://crates.io/crates/jc-rs",
  npm: "https://www.npmjs.com/package/jc-rs-wasm",
  releases: "https://github.com/OlegSotnikov/jc-rs/releases",
} as const;

import benchmarkData from "@/data/benchmarks.json";

/**
 * Measured by `ci/bench-vs-jc.sh`, which writes `data/benchmarks.json`.
 *
 * These numbers used to be typed here, and again in the compare page, and again
 * in the README. All three drifted, and the two on the site were two releases
 * out of date while claiming to describe the current one. Nothing on this site
 * is hand-written data; this file was the exception, and is not any more.
 */
export const benchmarks = benchmarkData.rows;

/** How the numbers above were obtained, for the caption under the chart. */
export const benchmarkMeta = {
  method: benchmarkData.method,
  jcVersion: benchmarkData.jcVersion,
  python: benchmarkData.python,
  platform: benchmarkData.platform,
  measured: benchmarkData.measured,
} as const;

export const install = [
  { label: "Cargo, prebuilt", cmd: "cargo binstall jc-rs" },
  { label: "Cargo, from source", cmd: "cargo install jc-rs" },
  { label: "Homebrew", cmd: "brew install OlegSotnikov/tap/jc-rs" },
  { label: "Docker", cmd: "docker pull appmasterio/jc-rs" },
  { label: "npm, WebAssembly", cmd: "npm install jc-rs-wasm" },
] as const;
