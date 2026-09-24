#!/usr/bin/env bash
# Pre-push wrapper around scripts/test.sh. Lefthook's pre-push job runs this
# with the remote name as $1 and git's ref lines on stdin:
#
#   <local ref> <local sha> <remote ref> <remote sha>
#
# The tests are skipped only when every pushed ref leaves the code unchanged
# relative to a commit that has already been through the gates, meaning every
# changed path is on the docs-only allowlist below. Anything the script cannot
# account for runs the tests. CI runs the full suite on every PR and every push
# to main regardless. See "Pre-commit and pre-push hooks" in docs/QUALITY.md.
#
# FORCE_TESTS=1 git push ...   always runs the tests.
#
# The functions can be sourced for testing without running the hook:
#   source scripts/pre-push-test.sh; docs_only_push origin < refs.txt
set -euo pipefail

ZERO_SHA_PATTERN='^0+$'

# True when a path cannot affect a build or a test: markdown under docs/,
# plans/ or review/, and the top-level TODO.md. README.md is not on the list:
# crates/clauvolution_app/src/cli.rs reads it with include_str! to check the
# flag table. unsafe_includes below refuses the skip if any crate starts
# reading a path this function accepts.
is_docs_path() {
  case "$1" in
    TODO.md) return 0 ;;
    docs/*.md | plans/*.md | review/*.md) return 0 ;;
    *) return 1 ;;
  esac
}

# Collapses "." and ".." segments in a relative path without touching the
# filesystem. Prints nothing when the path climbs above the repository root.
normalize_path() {
  local part out=()
  local IFS=/
  for part in $1; do
    case "$part" in
      "" | .) ;;
      ..)
        ((${#out[@]} > 0)) || return 1
        unset 'out[${#out[@]}-1]'
        ;;
      *) out+=("$part") ;;
    esac
  done
  echo "${out[*]}"
}

# Reads "<file>:<line text>" lines, as grep -rn prints them minus the line
# number, and prints one line for each include_str!/include_bytes! that makes
# the allowlist unsafe: its target is a path is_docs_path accepts, or it has
# no string literal on the same line to resolve (a multi-line call, concat!,
# env!), which the check cannot rule out.
unsafe_includes_from() {
  local file text literal target
  local literal_pattern='include_(str|bytes)!\([[:space:]]*"([^"]+)"'
  while IFS= read -r line; do
    file="${line%%:*}"
    text="${line#*:}"
    if [[ ! "$text" =~ $literal_pattern ]]; then
      echo "$file: include without a same-line literal: $text"
      continue
    fi
    literal="${BASH_REMATCH[2]}"
    target="$(normalize_path "$(dirname "$file")/$literal")" || continue
    if is_docs_path "$target"; then
      echo "$file: includes $target"
    fi
  done
}

# Scans crate sources for compile-time reads of allowlisted paths. Prints
# each offender and succeeds when there is at least one.
unsafe_includes() {
  local found
  found="$(grep -rnE --include='*.rs' 'include_(str|bytes)!' crates 2>/dev/null |
    sed -E 's/^([^:]+):[0-9]+:/\1:/' | unsafe_includes_from)"
  [[ -n "$found" ]] || return 1
  echo "$found"
}

# Prints the commit to diff a pushed ref against, or fails when there is none
# the script can trust. An existing remote ref diffs against its old tip; a
# new ref diffs against its merge-base with the remote's main branch.
push_base() {
  local remote="$1" local_sha="$2" remote_sha="$3"
  if [[ "$remote_sha" =~ $ZERO_SHA_PATTERN ]]; then
    git merge-base "$local_sha" "refs/remotes/$remote/main" 2>/dev/null
  else
    git cat-file -e "${remote_sha}^{commit}" 2>/dev/null || return 1
    echo "$remote_sha"
  fi
}

# Reads pre-push ref lines on stdin. Succeeds only when at least one line was
# read and every pushed ref changes docs-only paths; prints the reason for the
# verdict on stderr either way.
docs_only_push() {
  local remote="${1:-origin}"
  local local_ref local_sha remote_ref remote_sha base path
  local lines=0
  while read -r local_ref local_sha remote_ref remote_sha || [[ -n "${local_ref:-}" ]]; do
    [[ -z "${local_ref}${local_sha}" ]] && continue
    lines=$((lines + 1))
    if [[ -z "$remote_sha" ]]; then
      echo "pre-push: malformed ref line '$local_ref $local_sha $remote_ref'" >&2
      return 1
    fi
    # Deleting a remote ref pushes no code.
    if [[ "$local_sha" =~ $ZERO_SHA_PATTERN ]]; then
      continue
    fi
    if ! base="$(push_base "$remote" "$local_sha" "$remote_sha")" || [[ -z "$base" ]]; then
      echo "pre-push: no known base for $local_ref -> $remote_ref" >&2
      return 1
    fi
    # --no-renames lists both sides of a rename, so moving code into docs/
    # still shows the code path.
    local paths
    if ! paths="$(git diff --no-renames --name-only -z "$base" "$local_sha" | tr '\0' '\n')"; then
      echo "pre-push: could not diff $base..$local_sha" >&2
      return 1
    fi
    while IFS= read -r path; do
      [[ -z "$path" ]] && continue
      if ! is_docs_path "$path"; then
        echo "pre-push: $local_ref changes $path" >&2
        return 1
      fi
    done <<<"$paths"
  done
  if ((lines == 0)); then
    echo "pre-push: no refs on stdin" >&2
    return 1
  fi
  local offenders line
  if offenders="$(unsafe_includes)"; then
    echo "pre-push: a crate reads an allowlisted path at compile time, so the" >&2
    echo "pre-push: docs-only skip is unsafe; drop it from is_docs_path:" >&2
    while IFS= read -r line; do
      echo "pre-push:   $line" >&2
    done <<<"$offenders"
    return 1
  fi
  echo "pre-push: no pushed ref changes code" >&2
  return 0
}

main() {
  cd "$(dirname "$0")/.."
  if [[ "${FORCE_TESTS:-0}" != "0" ]]; then
    echo "pre-push: FORCE_TESTS set, running tests" >&2
  elif docs_only_push "${1:-origin}"; then
    echo "pre-push: skipping scripts/test.sh (CI still runs it); FORCE_TESTS=1 to run it" >&2
    exit 0
  fi
  exec scripts/test.sh
}

if [[ "${BASH_SOURCE[0]}" == "$0" ]]; then
  main "$@"
fi
