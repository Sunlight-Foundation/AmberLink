#!/usr/bin/env bash
# tools/watch.sh - rebuild + rerun on every *.amb change (Linux/macOS/Git Bash).
# Usage: make watch file=main.amb   (Ctrl+C to stop)
set -u
cd "$(dirname "$0")/.." || exit 1
file="${1:-}"
[ -z "$file" ] && { echo "Usage: make watch file=main.amb"; exit 1; }

run_file() {
  if [[ "$file" == *.amb ]]; then
    bin/ambc "$file" || { echo "[watch] compile failed"; return; }
    bin/avm "${file%.amb}.amc"
  else
    bin/avm "$file"
  fi
}

echo "[watch] watching *.amb under $PWD (Ctrl+C to stop)"
last=$(find . -name '*.amb' -type f | sort | xargs cksum 2>/dev/null)
run_file
while true; do
  sleep 2
  cur=$(find . -name '*.amb' -type f | sort | xargs cksum 2>/dev/null)
  if [ "$cur" != "$last" ]; then last="$cur"; echo "[watch] change detected"; run_file; fi
done
