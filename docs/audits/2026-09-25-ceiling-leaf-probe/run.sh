#!/usr/bin/env bash
# Usage: run.sh <setting-dir> <seed> [flags...]
# Runs one 15000-tick headless simulation from the repo root and writes
# seed<S>.txt (summary) and seed<S>.csv (1 Hz history) into
# docs/audits/2026-09-25-ceiling-leaf-probe/<setting-dir>/.
set -u
base="docs/audits/2026-09-25-ceiling-leaf-probe"
dir="$base/$1"
seed="$2"
shift 2
mkdir -p "$dir"
echo "$(date +%H:%M:%S) start $(basename "$dir") seed$seed" >> "$base/probe-run.log"
./target/release/clauvolution --headless 15000 --seed "$seed" "$@" \
  --dump-history "$dir/seed$seed.csv" > "$dir/seed$seed.txt" 2>&1
status=$?
echo "$(date +%H:%M:%S) done  $(basename "$dir") seed$seed (exit $status)" >> "$base/probe-run.log"
