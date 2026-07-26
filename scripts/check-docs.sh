#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

required_files=(
  README.md
  docs/README.md
  docs/quickstart.md
  CONTRIBUTING.md
  SECURITY.md
  SUPPORT.md
  CODE_OF_CONDUCT.md
  CHANGELOG.md
)

for file in "${required_files[@]}"; do
  if [[ ! -f "$file" ]]; then
    echo "missing required documentation: $file" >&2
    exit 1
  fi
done

for script in scripts/*.sh; do
  bash -n "$script"
done

# This command used to appear in installation guidance but only works before
# packaging. Keep the public instructions on the installed CLI path.
if rg -n 'target/release/rationale health' README.md docs; then
  echo "stale source-tree health command found in public documentation" >&2
  exit 1
fi

if rg -n 'dogfood\.7|releases/latest/download/rationale-installer\.sh' site/src; then
  echo "stale or floating Rationale installer reference found in the preview site" >&2
  exit 1
fi

git diff --check
echo "documentation checks passed"
