#!/usr/bin/env bash
# Genera las capturas actuales (snapshot_demo) y las copia como baseline.
# Guía: docs/dev/12-modelo-appkit.md, Paso 0.
# Verificar después de cada paso:
#   cargo test -- --ignored snapshot_demo && scripts/snapshot-baseline.sh --check
set -euo pipefail
cd "$(dirname "$0")/.."
if [[ "${1:-}" == "--check" ]]; then
  # Compara target/snapshots con la baseline guardada (sin regenerar la baseline).
  cd target/snapshots
  shasum -a 256 -c ../../snapshots/baseline/SHA256SUMS
  exit
fi
cargo test -- --ignored snapshot_demo
mkdir -p snapshots/baseline
cp target/snapshots/*.bmp snapshots/baseline/
(cd snapshots/baseline && shasum -a 256 *.bmp > SHA256SUMS)
echo "Baseline en snapshots/baseline ($(ls snapshots/baseline/*.bmp | wc -l | tr -d ' ') imágenes)"
