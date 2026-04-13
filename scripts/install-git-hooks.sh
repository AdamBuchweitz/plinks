#!/usr/bin/env bash

set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
legacy_hooks_dir=$(git -C "${repo_root}" rev-parse --git-path hooks)

if [[ "${legacy_hooks_dir}" != /* ]]; then
  legacy_hooks_dir="${repo_root}/${legacy_hooks_dir}"
fi

if [[ -d "${legacy_hooks_dir}" ]]; then
  mapfile -t legacy_hooks < <(
    find "${legacy_hooks_dir}" -maxdepth 1 -type f -perm -u+x ! -name "*.sample" -print \
      | sed 's|.*/||' \
      | sort
  )

  if [[ ${#legacy_hooks[@]} -gt 0 ]]; then
    echo "Refusing to set core.hooksPath because executable legacy hooks already exist in .git/hooks:" >&2
    printf '  - %s\n' "${legacy_hooks[@]}" >&2
    echo "Migrate or back up those hooks before rerunning this installer." >&2
    exit 1
  fi
fi

git -C "${repo_root}" config core.hooksPath .githooks

echo "Configured Git hooks path to .githooks"
echo "Pre-commit hook will now run repository linters before each commit."
