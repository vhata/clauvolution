#!/usr/bin/env bash
# Format gate: fail if any file in the workspace differs from rustfmt output.
# Run from anywhere; exits non-zero on any formatting diff.
set -euo pipefail
cd "$(dirname "$0")/.."

cargo fmt --all -- --check
