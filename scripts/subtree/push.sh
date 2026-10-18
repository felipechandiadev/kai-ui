#!/usr/bin/env bash
# Push monorepo prefix history to the standalone subtree repository.
set -euo pipefail

source "$(dirname "$0")/lib.sh"

package="${1:-}"
resolve_package "$package"
cd_repo_root

prefix="$(prefix_for "$package")"
remote="$(remote_for "$package")"

echo "git subtree push --prefix=$prefix $remote $KAI_UI_SUBTREE_BRANCH"
git subtree push --prefix="$prefix" "$remote" "$KAI_UI_SUBTREE_BRANCH"
