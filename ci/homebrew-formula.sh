#!/usr/bin/env bash
# Fill packaging/homebrew/jc-rs.rb.tmpl in from a published release.
#
# Checksums come from the release's own SHA256SUMS rather than from local
# builds, so the formula describes the artefacts people will actually download.
#
#   make homebrew-formula            # latest release
#   TAG=v0.2.0 make homebrew-formula
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."

REPO="${REPO:-OlegSotnikov/jc-rs}"
TAG="${TAG:-$(curl -sfL "https://api.github.com/repos/${REPO}/releases/latest" \
              | python3 -c 'import json,sys; print(json.load(sys.stdin)["tag_name"])')}"
OUT="${OUT:-Formula/jc-rs.rb}"

echo "generating $OUT for $TAG" >&2
sums=$(curl -sfL "https://github.com/${REPO}/releases/download/${TAG}/SHA256SUMS")

sha() {
  printf '%s\n' "$sums" | awk -v n="jc-rs-${TAG}-$1.tar.gz" '$2 == n || $2 == "*" n {print $1}'
}

mkdir -p "$(dirname "$OUT")"
sed \
  -e "s/__VERSION__/${TAG#v}/g" \
  -e "s/__SHA_DARWIN_ARM64__/$(sha aarch64-apple-darwin)/" \
  -e "s/__SHA_DARWIN_X86_64__/$(sha x86_64-apple-darwin)/" \
  -e "s/__SHA_LINUX_ARM64__/$(sha aarch64-unknown-linux-musl)/" \
  -e "s/__SHA_LINUX_X86_64__/$(sha x86_64-unknown-linux-musl)/" \
  packaging/homebrew/jc-rs.rb.tmpl > "$OUT"

if grep -q "__" "$OUT"; then
  echo "a placeholder was left unfilled — is $TAG published with all five archives?" >&2
  grep -n "__" "$OUT" >&2
  exit 1
fi
echo "wrote $OUT" >&2
