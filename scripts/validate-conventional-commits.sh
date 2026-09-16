#!/usr/bin/env bash
set -euo pipefail

# Validates the first line of each commit subject in a git revision range.
# See CONTRIBUTING.md and https://www.conventionalcommits.org/

usage() {
  echo "usage: $0 <rev-range>" >&2
  echo "example: $0 origin/main..HEAD" >&2
  exit 2
}

if [[ $# -ne 1 ]]; then
  usage
fi

range="$1"

if ! git rev-parse --verify "${range%%..*}" >/dev/null 2>&1; then
  echo "validate-conventional-commits: invalid or empty range: $range" >&2
  exit 1
fi

pattern='^(feat|fix|docs|style|refactor|perf|test|build|ci|chore|revert)(\([a-z0-9][a-z0-9._/-]*\))?!?: .+'

failed=0
while IFS= read -r subject; do
  [[ -z "$subject" ]] && continue
  if ! [[ "$subject" =~ $pattern ]]; then
    echo "non-conventional commit subject: $subject" >&2
    failed=1
  fi
done < <(git log --format=%s --no-merges "$range")

if [[ "$failed" -ne 0 ]]; then
  echo >&2
  echo "expected format: type(scope): description" >&2
  echo "types: feat, fix, docs, style, refactor, perf, test, build, ci, chore, revert" >&2
  echo "see CONTRIBUTING.md" >&2
  exit 1
fi

echo "conventional commits OK ($range)"
