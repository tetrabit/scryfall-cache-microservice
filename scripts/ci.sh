#!/usr/bin/env bash
set -euo pipefail

root_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

run() {
  echo
  echo "==> $*"
  "$@"
}

cd "$root_dir"

echo "Running local checks in: $root_dir"

run cargo fmt --check
run cargo test

if [[ -d "admin-panel" ]]; then
  # admin-panel is optional for pure-backend contributors.
  # Skip if package.json is missing.
  if [[ -f "admin-panel/package.json" ]]; then
    if [[ -f "admin-panel/package-lock.json" ]]; then
      run npm --prefix admin-panel ci
    else
      run npm --prefix admin-panel install
    fi
    run npm --prefix admin-panel run lint
    run npm --prefix admin-panel run build
  fi
fi

echo
echo "OK"
