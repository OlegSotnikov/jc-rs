// Runs the built npm package under Node and asserts the JS surface.
//
// This exists because `cargo build --target wasm32` and even `wasm-pack build`
// both passed while every record came back as an empty object: serde-wasm-bindgen
// maps a Rust map to a JS `Map` unless told otherwise, so `record.status` was
// undefined and `JSON.stringify` printed `{}`. Only running it caught that.
//
//   make wasm && node crates/jc-rs-wasm/tests/smoke.mjs
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const pkg = join(dirname(fileURLToPath(import.meta.url)), "..", "pkg");
const { default: init, parse, parseJson, parseRaw, parsers, parserInfo, version, StreamSession } =
  await import(join(pkg, "jc-rs.js"));

await init({ module_or_path: await readFile(join(pkg, "jc-rs_bg.wasm")) });

// Records are plain objects, not JS Maps.
const df = `Filesystem     1K-blocks    Used Available Use% Mounted on
devtmpfs         1918816       0   1918816   0% /dev`;
const [row] = parse("df", df);
assert.equal(row.filesystem, "devtmpfs", "field access works");
assert.equal(row.use_percent, 0, "numbers stay numbers");
assert.equal(JSON.parse(JSON.stringify(row)).filesystem, "devtmpfs", "survives stringify");

// The website consumes JSON text rather than round-tripping through JS values:
// null fields must remain present, and parser conversions from strings such as
// `100%` and `on` must be identical to the CLI schema.
const sparseDf = `Filesystem    Type 1024-blocks Used Available Capacity Mounted on
proc          proc           0    0         0        - /proc`;
const [sparseRow] = JSON.parse(parseJson("df", sparseDf));
assert.ok(Object.hasOwn(sparseRow, "capacity_percent"), "null field is not omitted");
assert.equal(sparseRow.capacity_percent, null);
assert.match(parseJson("df", sparseDf), /"capacity_percent": null/);

const amixer = `Simple mixer control 'Master',0
  Capabilities: pvolume pvolume-joined pswitch pswitch-joined
  Playback channels: Mono
  Limits: Playback 0 - 87
  Mono: Playback 87 [100%] [0.00dB] [on]`;
assert.deepEqual(JSON.parse(parseJson("amixer", amixer)), parse("amixer", amixer));

// JSON.parse would round this to a JavaScript Number. The text API preserves
// the exact CLI integer instead.
const procStat = await readFile(
  join(dirname(fileURLToPath(import.meta.url)), "../../../tests/fixtures/linux-proc/pid_stat"),
  "utf8",
);
assert.match(parseJson("proc_pid_stat", procStat), /"rsslim": 18446744073709551615/);

// Any spelling of the name.
assert.deepEqual(parse("--df", df), parse("df", df));

// -r returns the shape before jc's processing.
const env = "SHELL=/bin/zsh\nTERM=xterm\n";
assert.deepEqual(parse("env", env), [
  { name: "SHELL", value: "/bin/zsh" },
  { name: "TERM", value: "xterm" },
]);
assert.deepEqual(parseRaw("env", env), { SHELL: "/bin/zsh", TERM: "xterm" });

// Streaming, a line at a time.
const session = new StreamSession("clf_s");
const record = session.parseLine(
  '127.0.0.1 - frank [10/Oct/2000:13:55:36 -0700] "GET /a HTTP/1.0" 200 512');
assert.equal(record.status, 200);
assert.equal(record.host, "127.0.0.1");
assert.equal(session.finalize(), undefined, "clf_s buffers nothing");
assert.throws(() => new StreamSession("df"), /not a streaming parser/);

// Metadata.
assert.ok(parsers().length > 150, `only ${parsers().length} parsers`);
assert.ok(parsers().includes("git_log"));
assert.equal(parserInfo("nope"), undefined);
assert.match(parserInfo("ps").description, /ps/);
assert.match(version(), /^\d+\.\d+\.\d+$/);

// Unknown parser throws rather than returning nothing.
assert.throws(() => parse("nope", "x"), /no parser named/);

console.log("wasm smoke test passed");
