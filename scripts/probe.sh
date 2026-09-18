#!/usr/bin/env bash
# Probe: one long headless run with a history dump, failing on a non-zero
# exit or an extinct population; appends a markdown row to OUT_DIR/summary-row.md.
#
# Usage: scripts/probe.sh SEED TICKS OUT_DIR [LABEL]
# LABEL defaults to seed<SEED> and names the .txt and .csv files, so two
# runs of the same seed can live side by side in one OUT_DIR.
set -euo pipefail
cd "$(dirname "$0")/.."

SEED=${1:?usage: probe.sh SEED TICKS OUT_DIR [LABEL]}
TICKS=${2:?usage: probe.sh SEED TICKS OUT_DIR [LABEL]}
OUT_DIR=${3:?usage: probe.sh SEED TICKS OUT_DIR [LABEL]}
LABEL=${4:-seed$SEED}
BIN=target/release/clauvolution

if [ ! -x "$BIN" ]; then
  cargo build --release
fi

mkdir -p "$OUT_DIR"
SUMMARY="$OUT_DIR/$LABEL.txt"
HISTORY="$OUT_DIR/$LABEL.csv"
ROW="$OUT_DIR/summary-row.md"

echo "probe: $LABEL, seed $SEED, $TICKS ticks -> $SUMMARY"
set +e
"$BIN" --headless "$TICKS" --seed "$SEED" --dump-history "$HISTORY" > "$SUMMARY" 2>&1
status=$?
set -e

# Pull one integer from a "Label:  N" summary line; empty if absent.
field() {
  sed -n "s/^ *$1: *\([0-9][0-9]*\).*/\1/p" "$SUMMARY" | tail -n 1
}

population=$(field 'Total organisms (final)')
species=$(field 'Species (final)')
plants=$(field 'Plants')
foragers=$(field 'Foragers')
predators=$(field 'Predators')

if [ "$status" -ne 0 ]; then
  verdict="exit $status"
elif [ -z "$population" ]; then
  verdict="no summary"
elif [ "$population" -eq 0 ]; then
  verdict="extinct"
else
  verdict="ok"
fi

echo "| $LABEL | $SEED | $TICKS | ${population:-?} | ${plants:-?} | ${foragers:-?} | ${predators:-?} | ${species:-?} | $verdict |" >> "$ROW"

if [ "$verdict" != "ok" ]; then
  echo "probe: FAIL ($LABEL: $verdict); see $SUMMARY" >&2
  exit 1
fi
echo "probe: PASS ($LABEL: population $population, $species species)"
