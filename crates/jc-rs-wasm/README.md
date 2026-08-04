# jc-rs-wasm

jc-rs compiled to WebAssembly: convert the output of ~236 command-line tools,
file formats and strings to JSON, in the browser or in Node.

```js
import init, { parse, parsers, StreamSession } from "jc-rs-wasm";

await init();

parse("df", dfOutput);          // -> [{ filesystem: "devtmpfs", ... }]
parse("--git-log", gitLog);     // any spelling of the name works
parsers();                      // -> ["airport", "arp", "blkid", ...]

// Streaming parsers take a line at a time.
const session = new StreamSession("clf_s");
for (const line of lines) {
  const record = session.parseLine(line);
  if (record !== undefined) handle(record);
}
handleMaybe(session.finalize());
```

`parseRaw(name, input)` is jc's `-r`: the structure before the conversions that
shape it to jc's schema.

Everything is text in, structured data out — no I/O, no filesystem, no process
spawning. That is what makes the whole parser set portable here at all.

Compatibility with [jc](https://github.com/kellyjonbrazil/jc) is **100%** of its
fixture corpus, measured by `make differential` and published whatever it says.
CI fails below 100%.

Source, the compatibility report and what is left to do:
<https://github.com/OlegSotnikov/jc-rs>

MIT.
