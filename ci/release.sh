#!/usr/bin/env bash
# Cut a release: bump the version everywhere, commit, tag, push.
#
# Pushing the tag is what publishes -- GitHub Release, crates.io, Docker Hub,
# the Homebrew tap and npm all fire from it and none of them stops to ask.
# So everything that could make the release wrong is checked here, before the
# tag exists, while it still costs nothing to fix.
#
#   ./ci/release.sh 0.2.0
#
set -euo pipefail

VERSION="${1:-}"
FORCE="${FORCE:-0}"

die() { echo "release: $*" >&2; exit 1; }

[[ -n "$VERSION" ]] || die "usage: ./ci/release.sh X.Y.Z  (no leading v)"
[[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?$ ]] \
  || die "'$VERSION' is not X.Y.Z -- and no leading 'v', the tag gets that"

cd "$(dirname "$0")/.."
TAG="v$VERSION"

# --- everything that must be true before a tag exists ------------------------

branch=$(git rev-parse --abbrev-ref HEAD)
[[ "$branch" == "master" ]] || die "on '$branch', not master"

git diff --quiet && git diff --cached --quiet \
  || die "working tree is dirty -- commit or stash first"

git fetch --quiet origin master --tags
[[ "$(git rev-parse HEAD)" == "$(git rev-parse origin/master)" ]] \
  || die "HEAD and origin/master differ -- pull or push first"

# The mistake this exists to prevent: a stale local tag from an abandoned
# attempt still points at an old commit, `git push` sends it, and the release
# builds something other than what you are looking at.
if git rev-parse -q --verify "refs/tags/$TAG" >/dev/null; then
  die "local tag $TAG already exists (at $(git rev-parse --short "$TAG"^{commit}))
     if it is stale: git tag -d $TAG"
fi
if git ls-remote --exit-code --tags origin "$TAG" >/dev/null 2>&1; then
  die "$TAG is already published -- versions are immutable, pick the next one"
fi

current=$(sed -n 's/^version = "\(.*\)"$/\1/p' Cargo.toml | head -1)
[[ "$VERSION" != "$current" ]] || die "already at $VERSION"

# CI on this exact commit, if we can see it. Advisory: gh may be absent or
# logged out, and a release should not be blocked on that.
if command -v gh >/dev/null 2>&1; then
  conclusion=$(gh run list --branch master --workflow ci.yml --limit 20 \
    --json headSha,conclusion,status \
    --jq "[.[] | select(.headSha == \"$(git rev-parse HEAD)\")] | first
          | if . == null then \"none\"
            elif .status != \"completed\" then \"running\"
            else .conclusion end" 2>/dev/null || echo "unknown")
  case "$conclusion" in
    success) echo "ci: green on $(git rev-parse --short HEAD)" ;;
    unknown|none) echo "ci: no run found for this commit -- not checked" ;;
    *)
      echo "ci: $conclusion on $(git rev-parse --short HEAD)" >&2
      [[ "$FORCE" == "1" ]] || die "refusing to release a commit CI does not call green
     override with: FORCE=1 ./ci/release.sh $VERSION"
      ;;
  esac
fi

# --- the bump ----------------------------------------------------------------

# The workspace version, plus the version pin on every path dependency. Those
# pins are what crates.io resolves against, so a missed one publishes a crate
# that depends on the previous release of its sibling.
sed -i "0,/^version = \"$current\"$/s//version = \"$VERSION\"/" Cargo.toml
find crates -name Cargo.toml -exec \
  sed -i "s|\(path = \"\.\./jc-rs[a-z-]*\", version = \)\"$current\"|\1\"$VERSION\"|g" {} +

remaining=$(grep -rn "version = \"$current\"" --include=Cargo.toml . | grep -v '^./target' || true)
[[ -z "$remaining" ]] || die "version $current still pinned somewhere:
$remaining"

cargo update --workspace --quiet

echo
git --no-pager diff --stat
echo
read -rp "tag and push $TAG? [y/N] " reply
[[ "$reply" == "y" || "$reply" == "Y" ]] || { git checkout -- . && die "aborted, bump reverted"; }

git commit -aqm "Release $TAG"
git tag -a "$TAG" -m "$TAG"
git push -q origin master "$TAG"

echo "pushed $TAG -- release and publish-crates are running:"
echo "  https://github.com/OlegSotnikov/jc-rs/actions"
