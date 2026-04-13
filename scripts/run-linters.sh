#!/usr/bin/env bash

set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)

cd "${repo_root}"

echo "Checking formatting"
cargo fmt --check

echo "Running clippy"
cargo clippy --all-targets -- -D warnings
