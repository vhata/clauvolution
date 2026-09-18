#!/usr/bin/env bash
# Test gate: build and run every test target in the workspace.
# Run from anywhere; exits non-zero if any test fails to build or pass.
set -euo pipefail
cd "$(dirname "$0")/.."

cargo test --workspace
