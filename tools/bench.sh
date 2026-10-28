#!/usr/bin/env bash
# tools/bench.sh — run all benchmarks (Linux/macOS/Git Bash).
# Each bench self-times via clock() and prints result + seconds.
# Usage: make bench   (or: bash tools/bench.sh)
set -u
cd "$(dirname "$0")/.." || exit 1
if [ ! -x "bin/ambc" ] || [ ! -x "bin/avm" ]; then
  echo "ERROR: toolchain not built. Run 'make init' first."
  exit 1
fi
fail=0
for f in bench/*.amb; do
  echo "== $(basename "$f")"
  if ! bin/ambc "$f" >/dev/null 2>&1; then echo "$f : COMPILE FAIL"; fail=$((fail+1)); continue; fi
  if ! bin/avm "${f%.amb}.amc"; then echo "$f : RUN FAIL"; fail=$((fail+1)); fi
done
[ "$fail" -eq 0 ]
