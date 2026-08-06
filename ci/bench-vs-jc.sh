#!/usr/bin/env bash
# Time jc-rs against jc on the same inputs, on this machine, with one harness
# for both. Prints a markdown table.
#
# The speed claims in the README come from here, and anyone can re-run it.
#
# Both sides are invoked the way a user would. `python3 -m jc` includes the
# interpreter start-up that is most of what this comparison is about, and is
# what `jc` on your PATH does too.
set -uo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."

BIN="${BIN:-./target/release/jc-rs}"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

if [ ! -x "$BIN" ]; then
  echo "no release binary at $BIN; run: make build" >&2
  exit 1
fi
if [ ! -d jc/jc ]; then
  echo "jc submodule not checked out; run: make submodule" >&2
  exit 1
fi

# Two synthetic inputs, because the corpus has nothing large enough to say
# anything about throughput on repetitive data.
python3 - "$WORK" <<'PY'
import sys
work = sys.argv[1]
line = ('127.0.0.1 user-identifier frank [10/Oct/2000:13:55:36 -0700] '
        '"GET /apache_pb.gif HTTP/1.0" 200 2326\n')
with open(f"{work}/big.clf", "w") as f:
    f.write(line * 10000)
with open(f"{work}/rows.csv", "w") as f:
    f.write("a,b,c,d\n")
    for i in range(10000):
        f.write(f"{i},{i * 2},value{i},other{i}\n")
PY

# Fastest of N runs, not the median. Every run competes with whatever else the
# machine is doing, so slow runs measure the load and only the fastest measures
# the program. The median moves with the background; the minimum reproduces.
fastest() {
  local runs="$1" input="$2"; shift 2
  local best='' start end ms
  for _ in $(seq "$runs"); do
    start=$(date +%s%N)
    if [ -n "$input" ]; then
      "$@" < "$input" > /dev/null 2>&1
    else
      "$@" > /dev/null 2>&1
    fi
    end=$(date +%s%N)
    ms=$(( (end - start) / 1000000 ))
    if [ -z "$best" ] || [ "$ms" -lt "$best" ]; then best="$ms"; fi
  done
  echo "$best"
}

# Both sides are invoked identically: same wrapper, same redirection, same
# number of processes. An earlier version ran the rows that take input through
# an extra `bash -c` and the cold-start row without one, which charged every
# other row for a shell it never needed.
row() {
  local label="$1" runs="$2" args="$3" input="${4:-}"
  local jc_ms rs_ms
  jc_ms=$(fastest "$runs" "$input" env PYTHONPATH=jc python3 -m jc "$args")
  rs_ms=$(fastest "$runs" "$input" "$BIN" "$args")
  printf '| %-36s | %5s ms | %5s ms | **%sx** |\n' \
    "$label" "$jc_ms" "$rs_ms" \
    "$(python3 -c "print(f'{$jc_ms / max($rs_ms, 1):.1f}')")"
}

echo "| Scenario | jc | jc-rs | Speedup |"
echo "|---|---|---|---|"
row 'Cold start (`-v`)'                 15 -v
row '`ps aux`, 110 lines'               11 --ps            tests/fixtures/centos-7.7/ps-axu.out
# Two small inputs whose cost is per-record rather than per-byte: both sides
# carry a large pattern set here, so the row says something the throughput
# rows do not.
row '`traceroute`, 1.5 KB'              11 --traceroute    tests/fixtures/generic/traceroute1.out
row '`ifconfig`, 1.3 KB'                11 --ifconfig      tests/fixtures/centos-7.7/ifconfig.out
row '`clf`, 10,000 log lines'            7 --clf           "$WORK/big.clf"
row '`csv`, 10,000 rows'                 7 --csv           "$WORK/rows.csv"
row '`pkg-index-deb`, 1.5 MB'            5 --pkg-index-deb tests/fixtures/generic/pkg-index-deb.out
