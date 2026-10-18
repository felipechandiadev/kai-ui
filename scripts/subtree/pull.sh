#!/usr/bin/env bash
# Merge changes from the standalone repo back into the monorepo prefix.
set -euo pipefail

source "$(dirname "$0")/lib.sh"

package="${1:-}"
resolve_package "$package"
cd_repo_root

prefix="$(prefix_for "$package")"
remote="$(remote_for "$package")"

squash_flag=()
if [[ "${KAI_UI_SUBTREE_PULL_SQUASH:-1}" == "1" ]]; then
  squash_flag=(--squash)
fi

echo "git subtree pull --prefix=$prefix $remote $KAI_UI_SUBTREE_BRANCH ${squash_flag[*]}"
git subtree pull --prefix="$prefix" "$remote" "$KAI_UI_SUBTREE_BRANCH" "${squash_flag[@]}"
