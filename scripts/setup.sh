#!/usr/bin/env bash
# One-time developer setup: install lefthook's git hooks (pre-commit and
# pre-push, from lefthook.yml). Tells you how to get lefthook if it is missing.
set -euo pipefail
cd "$(dirname "$0")/.."

if ! command -v lefthook >/dev/null 2>&1; then
  cat >&2 <<'MSG'
setup: lefthook is not on PATH.

Install it with:
  brew install lefthook

Other platforms: https://github.com/evilmartians/lefthook
Then re-run scripts/setup.sh.
MSG
  exit 1
fi

lefthook install
echo
echo "Hooks now active (from lefthook.yml):"
echo "  pre-commit: scripts/fmt-check.sh and scripts/lint.sh, in parallel"
echo "  pre-push:   scripts/test.sh"
echo "Hooks are installed in the shared .git/hooks, so they cover every worktree."
