# Nothing but the static binary. No shell, no libc, no package manager —
# the release workflow drops a musl-linked jc-rs for each architecture next to
# this file and that is the entire image.
#
# `TARGETARCH` is set by buildx per platform, so one build produces amd64 and
# arm64 under a single tag and the daemon picks the right one. No emulation is
# involved: nothing runs at build time, and both binaries were already compiled
# natively by the release matrix.
FROM scratch

ARG TARGETARCH
COPY binaries/linux/${TARGETARCH}/jc-rs /jc-rs
COPY LICENSE /LICENSE

ENTRYPOINT ["/jc-rs"]
