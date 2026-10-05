#!/usr/bin/env bash
# Local entry point: runs the queue, link, format, lint, and test gates in that
# order (cheapest first, as in CI) and stops at the first failure, naming the
# gate that failed.
set -euo pipefail
cd "$(dirname "$0")/.."

gate() {
  local name=$1
  shift
  echo "==> $name"
  if ! "$@"; then
    echo "check: gate '$name' failed ($*)" >&2
    exit 1
  fi
}

gate queues scripts/workflow/check-queues.sh --strict
gate links scripts/workflow/check-links.sh
for name in fmt-check lint test; do
  gate "$name" "scripts/$name.sh"
done
echo "check: all gates passed"
