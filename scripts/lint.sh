#!/usr/bin/env bash
# Lint gate: clippy over every crate and target with warnings denied.
# Run from anywhere; exits non-zero on any warning or error.
set -euo pipefail
cd "$(dirname "$0")/.."

cargo clippy --workspace --all-targets -- -D warnings
