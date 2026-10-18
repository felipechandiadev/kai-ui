#!/usr/bin/env bash
# Shared helpers for kai-ui git subtree workflows (bash 3.2+).
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

KAI_UI_SUBTREE_WEB_REMOTE="${KAI_UI_SUBTREE_WEB_REMOTE:-https://github.com/felipechandiadev/kai-ui-web.git}"
KAI_UI_SUBTREE_DESKTOP_REMOTE="${KAI_UI_SUBTREE_DESKTOP_REMOTE:-https://github.com/felipechandiadev/kai-ui-desktop.git}"
KAI_UI_SUBTREE_BRANCH="${KAI_UI_SUBTREE_BRANCH:-main}"

usage_packages() {
  echo "web | desktop"
}

resolve_package() {
  local package="${1:-}"
  case "$package" in
    web | desktop) ;;
    *)
      echo "error: package must be one of: $(usage_packages)" >&2
      exit 1
      ;;
  esac
}

prefix_for() {
  case "$1" in
    web) echo "kai-ui-web" ;;
    desktop) echo "kai-ui-desktop" ;;
  esac
}

remote_name_for() {
  case "$1" in
    web) echo "subtree-web" ;;
    desktop) echo "subtree-desktop" ;;
  esac
}

remote_url_for() {
  case "$1" in
    web) echo "$KAI_UI_SUBTREE_WEB_REMOTE" ;;
    desktop) echo "$KAI_UI_SUBTREE_DESKTOP_REMOTE" ;;
  esac
}

cd_repo_root() {
  cd "$REPO_ROOT"
}

ensure_remote() {
  local package="$1"
  local name
  local url
  name="$(remote_name_for "$package")"
  url="$(remote_url_for "$package")"
  if git remote get-url "$name" &>/dev/null; then
    local current
    current="$(git remote get-url "$name")"
    if [[ "$current" != "$url" ]]; then
      echo "note: remote $name already points to $current (expected $url)" >&2
    fi
  else
    git remote add "$name" "$url"
    echo "added remote $name -> $url"
  fi
}

remote_for() {
  local package="$1"
  ensure_remote "$package"
  remote_name_for "$package"
}
