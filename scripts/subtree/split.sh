#!/usr/bin/env bash
# Create a local branch with only the prefix history (for inspection or manual push).
set -euo pipefail

source "$(dirname "$0")/lib.sh"

package="${1:-}"
resolve_package "$package"
cd_repo_root

prefix="$(prefix_for "$package")"
branch="subtree/${package}"

git branch -D "$branch" 2>/dev/null || true
git subtree split --prefix="$prefix" -b "$branch"
echo "branch $branch at $(git rev-parse --short "$branch")"
