#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

AMBC="bin/ambc"
AVM="bin/avm"
if [ ! -x "$AMBC" ] || [ ! -x "$AVM" ]; then
  echo "ERROR: toolchain not built. Run the regression harness first." >&2
  exit 1
fi

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

for name in basic_types_test factorial float_test fold_test hello hi match_test null_test test_static; do
  source="examples/$name.amb"
  bytecode="examples/$name.amc"
  native="$tmp/$name"

  "$AMBC" "$source" --native "$native" >/dev/null
  expected=$("$AVM" "$bytecode")
  actual=$("$native")

  if [ "$actual" != "$expected" ]; then
    printf 'AOT mismatch for %s\n--- VM ---\n%s\n--- native ---\n%s\n' "$name" "$expected" "$actual" >&2
    exit 1
  fi
  echo "$name : AOT matches AVM"
done

echo "AOT_DIFFERENTIAL_FAILURES=0"
