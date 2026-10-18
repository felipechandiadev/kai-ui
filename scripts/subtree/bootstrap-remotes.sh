#!/usr/bin/env bash
# Register subtree remotes (idempotent). Does not push.
set -euo pipefail

source "$(dirname "$0")/lib.sh"

cd_repo_root
ensure_remote web
ensure_remote desktop
echo "remotes ready: subtree-web, subtree-desktop"
