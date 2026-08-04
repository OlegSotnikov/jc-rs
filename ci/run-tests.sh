#!/usr/bin/env bash
# Run the test suite as a ratchet against ci/known-failures.txt.
#
# Plain `cargo test` is red right now and will stay red until M3 is done, which
# makes it useless as a signal. This wrapper turns it into a useful one:
#
#   new failure          -> build fails (a regression)
#   known failure fixed  -> build fails (delete the line, in the same commit)
#   known failure fails  -> fine, that is the current state
#
# TZ matters: jc's fixtures carry *_epoch fields computed in local time and its
# own runtests.sh pins PST8PDT. Any other zone and a pile of timestamp tests
# fail for a reason that has nothing to do with the code.
set -uo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

KNOWN_FILE="ci/known-failures.txt"
LOG="$(mktemp)"
trap 'rm -f "$LOG"' EXIT

echo "running the workspace test suite under TZ=PST8PDT"
TZ=PST8PDT cargo test --workspace --no-fail-fast --color never > "$LOG" 2>&1
echo "  (cargo exited $?)"

# Names of tests that actually failed, e.g. "network::ufw::tests::test_ufw".
# libtest prints them under a "failures:" block, one indented name per line.
actual="$(awk '
  /^failures:$/ { collecting = 1; next }
  collecting && /^    [A-Za-z_][A-Za-z0-9_:]*$/ { print substr($0, 5); next }
  collecting && !/^    / { collecting = 0 }
' "$LOG" | sort -u)"

known="$(grep -vE '^\s*(#|$)' "$KNOWN_FILE" | sort -u)"

new_failures="$(comm -23 <(printf '%s\n' "$actual") <(printf '%s\n' "$known") | grep -v '^$' || true)"
now_passing="$(comm -13 <(printf '%s\n' "$actual") <(printf '%s\n' "$known") | grep -v '^$' || true)"

# A compile error produces no "failures:" block at all. Do not let that read as
# "everything passed".
if ! grep -q '^test result:' "$LOG"; then
  echo
  echo "the suite did not run — build or link error:"
  tail -40 "$LOG"
  exit 1
fi

printf '\n%s\n' "----------------------------------------------------------------"
printf 'failing now : %s\n' "$(printf '%s\n' "$actual" | grep -c . || true)"
printf 'known        : %s\n' "$(printf '%s\n' "$known" | grep -c . || true)"

status=0

if [ -n "$new_failures" ]; then
  echo
  echo "NEW failures — these are regressions:"
  printf '  %s\n' $new_failures
  status=1
fi

if [ -n "$now_passing" ]; then
  echo
  echo "these are listed as known failures but PASS now."
  echo "remove them from $KNOWN_FILE in the commit that fixed them:"
  printf '  %s\n' $now_passing
  status=1
fi

if [ "$status" -eq 0 ]; then
  echo
  echo "no regressions. $(printf '%s\n' "$known" | grep -c .) known failures remain — see $KNOWN_FILE"
fi

exit "$status"
