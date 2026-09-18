#!/usr/bin/env bash
# Smoke gate: release build, then a short headless run that must exit 0 and
# end with a living population. Output is tee'd to ${SMOKE_OUT:-smoke.txt}.
set -euo pipefail
cd "$(dirname "$0")/.."

TICKS=${SMOKE_TICKS:-500}
SEED=${SMOKE_SEED:-42}
OUT=${SMOKE_OUT:-smoke.txt}
BIN=target/release/clauvolution

cargo build --release

# The summary is printed on stderr, so fold both streams into the capture.
# Keep the binary's exit code rather than tee's.
set +e
"$BIN" --headless "$TICKS" --seed "$SEED" 2>&1 | tee "$OUT"
status=${PIPESTATUS[0]}
set -e

if [ "$status" -ne 0 ]; then
  echo "smoke: FAIL (binary exited $status)" >&2
  exit 1
fi

population=$(sed -n 's/^Total organisms (final): *\([0-9][0-9]*\).*/\1/p' "$OUT" | tail -n 1)
if [ -z "$population" ]; then
  echo "smoke: FAIL (no 'Total organisms (final)' line in $OUT)" >&2
  exit 1
fi
if [ "$population" -eq 0 ]; then
  echo "smoke: FAIL (population is zero after $TICKS ticks, seed $SEED)" >&2
  exit 1
fi

echo "smoke: PASS (seed $SEED, $TICKS ticks, final population $population, summary in $OUT)"
