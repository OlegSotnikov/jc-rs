# jc-rs

Convert the output of command-line tools, file formats and strings to JSON — one static binary. This crate is the `jc-rs` CLI.

**Version 0.0.0 reserves the name; it is not a release.** jc-rs is pre-release
software: compatibility with [jc](https://github.com/kellyjonbrazil/jc) is
currently 86.1% of its fixture corpus, measured by `make differential` and
published whatever it says. The first usable release will be 0.1.0, cut when
that number reaches 100% and streaming output is line-by-line NDJSON.

Until then, do not depend on this.

Source, the compatibility report and what is left to do:
<https://github.com/OlegSotnikov/jc-rs>

MIT.
