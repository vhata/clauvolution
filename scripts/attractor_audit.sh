#!/usr/bin/env bash
# Run the attractor audit: every seed for TICKS ticks, REPEATS times, with
# the two runs of a seed placed in different batches so that same-seed runs
# never start simultaneously (runs launched together have come out
# bit-identical while runs launched apart have diverged). Writes one summary
# and one 1 Hz history CSV per run into OUT_DIR, then two simultaneous
# runs of PROBE_SEED as a determinism probe.
#
# Usage: scripts/attractor_audit.sh OUT_DIR [TICKS] [PARALLEL]
# Seeds and repeats are set below so audits stay comparable across time.
# EXTRA_ARGS="--bite-fraction 0.2" passes config overrides to every run and
# is recorded in README.txt.
set -euo pipefail

OUT_DIR=${1:?usage: attractor_audit.sh OUT_DIR [TICKS] [PARALLEL]}
TICKS=${2:-15000}
PARALLEL=${3:-4}
SEEDS=(1 2 3 42 314 7 99 1000)
REPEATS=2
PROBE_SEED=42
BIN=./target/release/clauvolution
EXTRA_ARGS=${EXTRA_ARGS:-}

mkdir -p "$OUT_DIR"
cargo build --release 2>&1 | grep -E '^error|Finished'
echo "commit $(git rev-parse --short HEAD), ticks $TICKS, seeds ${SEEDS[*]}, $REPEATS runs each, extra args: ${EXTRA_ARGS:-none}" | tee "$OUT_DIR/README.txt"

run_one() {
  local seed=$1 run=$2 label="seed$1-run$2"
  echo "$(date +%T) start $label"
  # shellcheck disable=SC2086
  "$BIN" --headless "$TICKS" --seed "$seed" --dump-history "$OUT_DIR/$label.csv" $EXTRA_ARGS \
    > "$OUT_DIR/$label.txt" 2>&1
  echo "$(date +%T) done  $label"
}

for run in $(seq 1 $REPEATS); do
  for ((i = 0; i < ${#SEEDS[@]}; i += PARALLEL)); do
    for seed in "${SEEDS[@]:i:PARALLEL}"; do
      run_one "$seed" "$run" &
    done
    wait
  done
done

echo "$(date +%T) determinism probe: two simultaneous runs of seed $PROBE_SEED"
run_one "$PROBE_SEED" probe-a &
run_one "$PROBE_SEED" probe-b &
wait
echo "$(date +%T) audit complete"
