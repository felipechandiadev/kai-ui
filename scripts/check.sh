#!/usr/bin/env bash
# Full check of the Rust crates: fmt, clippy (warnings are errors), tests
# in the three feature modes, docs, fixtures, kai-build and kai-cli.
# Prints one line per step and only shows the output of what fails.
#
#   scripts/check.sh          everything
#   scripts/check.sh quick    kai-ui-desktop with --features document only
set -uo pipefail

cd "$(dirname "$0")/.."
ROOT=$(pwd)
CARGO=(cargo +1.95)
LOG=$(mktemp -t kai-check)
FAILED=0

step() {
  local name=$1
  shift
  if "$@" >"$LOG" 2>&1; then
    printf 'ok    %s\n' "$name"
  else
    printf 'FALLA %s\n' "$name"
    grep -E '^(error|warning)|-->|panicked|FAILED|^test .* FAILED' "$LOG" | head -20 | sed 's/^/      /'
    FAILED=1
  fi
}

desktop() { (cd "$ROOT/kai-ui-desktop" && "${CARGO[@]}" "$@"); }

step "fmt" desktop fmt --check
step "clippy document" desktop clippy --locked --all-targets --features document -- -D warnings
step "test document" desktop test --locked --features document

if [[ "${1:-}" != "quick" ]]; then
  step "clippy" desktop clippy --locked --all-targets -- -D warnings
  step "test" desktop test --locked
  step "clippy mcp" desktop clippy --locked --all-targets --features mcp -- -D warnings
  step "test mcp" desktop test --locked --features mcp
  step "doc" env RUSTDOCFLAGS="-D warnings" bash -c "cd '$ROOT/kai-ui-desktop' && ${CARGO[*]} doc --locked --no-deps"
  step "fixture external-app" desktop check --locked --manifest-path fixtures/external-app/Cargo.toml --target-dir target
  step "fixture sections-app" desktop test --locked --manifest-path fixtures/sections-app/Cargo.toml --target-dir target
  for crate in kai-build kai-cli; do
    step "$crate fmt" bash -c "cd '$ROOT/$crate' && ${CARGO[*]} fmt --check"
    step "$crate clippy" bash -c "cd '$ROOT/$crate' && ${CARGO[*]} clippy --locked --all-targets -- -D warnings"
    step "$crate test" bash -c "cd '$ROOT/$crate' && ${CARGO[*]} test --locked"
  done
fi

rm -f "$LOG"
SIZE=$(du -sh "$ROOT/kai-ui-desktop/target" 2>/dev/null | cut -f1)
FREE=$(df -h "$ROOT" | awk 'NR==2 {print $4}')
echo "target ${SIZE:-0}, disco libre $FREE"
exit $FAILED
