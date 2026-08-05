# jc-rs

Convert the output of command-line tools, file formats and strings to JSON,
from one static binary. This crate is the `jc-rs` CLI.

Also available as a container:
[`appmasterio/jc-rs`](https://hub.docker.com/r/appmasterio/jc-rs) on Docker Hub,
a 2.3 MB `scratch` image for `linux/amd64` and `linux/arm64`.

```console
$ ps aux | docker run --rm -i appmasterio/jc-rs --ps | jq '.[0]'
```

Compatibility with [jc](https://github.com/kellyjonbrazil/jc) is **100%** of its
fixture corpus, measured by `make differential` and published whatever it says.
CI fails below 100%.

Source, the compatibility report and what is left to do:
<https://github.com/OlegSotnikov/jc-rs>

MIT.
