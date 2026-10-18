#!/usr/bin/env bash
# Attach an existing standalone repo as a subtree prefix (fresh clone / new integration).
set -euo pipefail

source "$(dirname "$0")/lib.sh"

package="${1:-}"
resolve_package "$package"
cd_repo_root

prefix="$(prefix_for "$package")"
remote="$(remote_for "$package")"

if [[ -e "$prefix" ]]; then
  echo "error: $prefix already exists; subtree add is for empty integration only" >&2
  exit 1
fi

squash_flag=()
if [[ "${KAI_UI_SUBTREE_ADD_SQUASH:-1}" == "1" ]]; then
  squash_flag=(--squash)
fi

echo "git subtree add --prefix=$prefix $remote $KAI_UI_SUBTREE_BRANCH ${squash_flag[*]}"
git subtree add --prefix="$prefix" "$remote" "$KAI_UI_SUBTREE_BRANCH" "${squash_flag[@]}"
