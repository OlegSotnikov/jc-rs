# jc-rs-parsers

Parser implementations for jc-rs — the reuse surface for other tools that need
command output as structured data.

```rust
let output = jc_rs_parsers::parse("df", df_output)?;

// Streaming parsers hand back a session you feed a line at a time.
let mut session = jc_rs_parsers::session("clf_s").unwrap();
while let Some(line) = lines.next() {
    if let Some(record) = session.parse_line(&line, true)? {
        handle(record);
    }
}
```

Parsers register themselves at link time, so depending on this crate is what
fills the registry; `parse`, `find`, `parsers` and `session` exist so you never
have to think about that.

**Version 0.0.0 reserves the name; it is not a release.** jc-rs is pre-release
software: compatibility with [jc](https://github.com/kellyjonbrazil/jc) is
currently 97.8% of its fixture corpus, measured by `make differential` and
published whatever it says. The first usable release will be 0.1.0, cut when
that number reaches 100%.

Until then, do not depend on this.

Source, the compatibility report and what is left to do:
<https://github.com/OlegSotnikov/jc-rs>

MIT.
