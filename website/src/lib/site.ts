export const site = {
  name: "jc-rs",
  origin: process.env.NEXT_PUBLIC_SITE_ORIGIN ?? "https://jc-rs.com",
  tagline: "Command output to JSON, from one static binary",
  repo: "https://github.com/OlegSotnikov/jc-rs",
  jc: "https://github.com/kellyjonbrazil/jc",
  dockerHub: "https://hub.docker.com/r/appmasterio/jc-rs",
  cratesIo: "https://crates.io/crates/jc-rs",
  npm: "https://www.npmjs.com/package/jc-rs-wasm",
  releases: "https://github.com/OlegSotnikov/jc-rs/releases",
} as const;

/** Measured by ci/bench-vs-jc.sh, median of 5-11 runs, Linux x86-64. */
export const benchmarks = [
  { scenario: "Cold start", detail: "jc-rs -v", jc: 138, rs: 7 },
  { scenario: "ps aux", detail: "110 lines", jc: 156, rs: 12 },
  { scenario: "csv", detail: "10,000 rows", jc: 200, rs: 41 },
  { scenario: "pkg-index-deb", detail: "1.5 MB", jc: 309, rs: 88 },
  { scenario: "clf", detail: "10,000 log lines", jc: 604, rs: 208 },
] as const;

export const install = [
  { label: "Cargo, prebuilt", cmd: "cargo binstall jc-rs" },
  { label: "Cargo, from source", cmd: "cargo install jc-rs" },
  { label: "Homebrew", cmd: "brew install OlegSotnikov/tap/jc-rs" },
  { label: "Docker", cmd: "docker pull appmasterio/jc-rs" },
  { label: "npm, WebAssembly", cmd: "npm install jc-rs-wasm" },
] as const;
