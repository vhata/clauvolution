#!/usr/bin/env bash
# Run the attractor audit: every seed for TICKS ticks, REPEATS times, at most
# PARALLEL runs at once. Headless runs are deterministic (see "Headless runs
# are deterministic" in docs/DECISIONS.md), so one run per seed is one
# trajectory and a second run of the same seed would only reproduce it.
# Writes one summary and one 1 Hz history CSV per run into OUT_DIR, then two
# runs of PROBE_SEED, launched together unless PARALLEL is 1, as a
# determinism probe that scripts/attractor_audit_summary.py compares.
#
# Exits non-zero if any run exits non-zero or ends without a final summary,
# after every run has finished; the failed runs are named on stderr. An
# extinct population is a result, not a failure, and is left to the reader.
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
REPEATS=1
PROBE_SEED=42
BIN=./target/release/clauvolution
EXTRA_ARGS=${EXTRA_ARGS:-}

mkdir -p "$OUT_DIR"
cargo build --release 2>&1 | grep -E '^error|Finished'
echo "commit $(git rev-parse --short HEAD), ticks $TICKS, seeds ${SEEDS[*]}, $REPEATS run(s) each, extra args: ${EXTRA_ARGS:-none}" | tee "$OUT_DIR/README.txt"

run_one() {
  local seed=$1 run=$2 label="seed$1-run$2" status=0
  echo "$(date +%T) start $label"
  # shellcheck disable=SC2086
  "$BIN" --headless "$TICKS" --seed "$seed" --dump-history "$OUT_DIR/$label.csv" $EXTRA_ARGS \
    > "$OUT_DIR/$label.txt" 2>&1 || status=$?
  if [ "$status" -ne 0 ]; then
    echo "$(date +%T) FAIL  $label (exit $status)"
    return "$status"
  fi
  if ! grep -q '^Total organisms (final):' "$OUT_DIR/$label.txt"; then
    echo "$(date +%T) FAIL  $label (no final summary)"
    return 1
  fi
  echo "$(date +%T) done  $label"
}

# Launch the "seed run" pairs given as arguments, PARALLEL at a time, and wait
# on each batch PID by PID so that every exit status is seen. Failed labels
# are appended to the global failed array.
failed=()
run_batches() {
  local jobs=("$@") i j job seed run pids labels
  for ((i = 0; i < ${#jobs[@]}; i += PARALLEL)); do
    pids=()
    labels=()
    for job in "${jobs[@]:i:PARALLEL}"; do
      read -r seed run <<< "$job"
      run_one "$seed" "$run" &
      pids+=($!)
      labels+=("seed$seed-run$run")
    done
    for j in "${!pids[@]}"; do
      wait "${pids[$j]}" || failed+=("${labels[$j]}")
    done
  done
}

audit_jobs=()
for run in $(seq 1 "$REPEATS"); do
  for seed in "${SEEDS[@]}"; do
    audit_jobs+=("$seed $run")
  done
done
run_batches "${audit_jobs[@]}"

echo "$(date +%T) determinism probe: two runs of seed $PROBE_SEED, simultaneous unless PARALLEL is 1"
run_batches "$PROBE_SEED probe-a" "$PROBE_SEED probe-b"

if [ ${#failed[@]} -gt 0 ]; then
  echo "$(date +%T) audit FAILED: ${#failed[@]} run(s) failed: ${failed[*]}; see their .txt files in $OUT_DIR" >&2
  exit 1
fi
echo "$(date +%T) audit complete"
