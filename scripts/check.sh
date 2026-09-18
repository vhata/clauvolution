#!/usr/bin/env bash
# Local entry point: runs the format, lint, and test gates in that order and
# stops at the first failure, naming the gate that failed.
set -euo pipefail
cd "$(dirname "$0")/.."

for gate in fmt-check lint test; do
  echo "==> $gate"
  if ! "scripts/$gate.sh"; then
    echo "check: gate '$gate' failed (scripts/$gate.sh)" >&2
    exit 1
  fi
done
echo "check: all gates passed"
