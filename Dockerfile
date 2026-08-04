# Nothing but the static binary. No shell, no libc, no package manager —
# the release workflow drops a musl-linked jc-rs next to this file and that is
# the entire image.
FROM scratch

COPY jc-rs /jc-rs
COPY LICENSE /LICENSE

ENTRYPOINT ["/jc-rs"]
