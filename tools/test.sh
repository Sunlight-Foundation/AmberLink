#!/usr/bin/env bash
# tools/test.sh - Amberlink regression harness (Linux/macOS/Git Bash).
# Compiles every example + stdlib file, runs every example, reports failures.
# Usage: make test   (or: bash tools/test.sh)
# Exit code: 0 = all green, 1 = any failure.
set -u
cd "$(dirname "$0")/.." || exit 1

AMBC="bin/ambc"
AVM="bin/avm"
if [ ! -x "$AMBC" ] || [ ! -x "$AVM" ]; then
  echo "ERROR: toolchain not built. Run 'make init' first."
  exit 1
fi

EXAMPLES="factorial hello hi basic_types_test float_test list_test test_init test_methods test_oop test_overload test_static test_visibility gc_test native_test file_io_test collections_test import_test archive_test net_test ffi_test threads_test fold_test"
STDLIB="stdlib/core.amb stdlib/io.amb stdlib/collections.amb stdlib/net.amb stdlib/ffi.amb"
fail=0

# net_test needs the local echo server; start it when python exists.
server=""
if command -v python3 >/dev/null 2>&1; then
  PY=python3
elif command -v python >/dev/null 2>&1; then
  PY=python
else
  echo "NOTE: python not found - net_test run step will be skipped (compile only)."
fi
if [ -n "${PY:-}" ]; then
  "$PY" examples/resources/echo_server.py & server=$!
  sleep 2
fi

for e in $EXAMPLES; do
  # --emit-ir also asserts the backend-IR decode/re-encode round-trip.
  if ! "$AMBC" "examples/$e.amb" --emit-ir >/dev/null 2>&1; then echo "$e : COMPILE FAIL"; fail=$((fail+1)); continue; fi
  if [ "$e" = "net_test" ] && [ -z "$server" ]; then echo "$e : compile OK (run skipped, no python)"; continue; fi
  if ! "$AVM" "examples/$e.amc" >/dev/null 2>&1; then echo "$e : RUN FAIL"; fail=$((fail+1)); else echo "$e : OK"; fi
done

for s in $STDLIB; do
  if ! "$AMBC" "$s" --emit-ir >/dev/null 2>&1; then echo "$s : COMPILE FAIL"; fail=$((fail+1)); else echo "$s : OK"; fi
done

if [ -n "$server" ]; then kill "$server" 2>/dev/null; fi

echo "FAILURES=$fail"
[ "$fail" -eq 0 ]
