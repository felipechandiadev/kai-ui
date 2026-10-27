#!/usr/bin/env bash
# Genera las capturas actuales (las de la demo, demo/tests/snapshots.rs, y
# las dos del renderer) y las copia como baseline.
# Guía: docs/dev/12-modelo-appkit.md, Paso 0.
# Verificar después de cada paso:
#   scripts/snapshot-baseline.sh --check
set -euo pipefail
cd "$(dirname "$0")/.."
# The demo's captures, and the two of the renderer (images and geometry).
shots() {
  cargo +1.95 test --manifest-path demo/Cargo.toml --target-dir target --test snapshots -- --ignored snapshot_demo
  cargo +1.95 test --lib -- --ignored images_draw_rounded_cropped_and_clipped \
    geometry_draws_antialiased_strokes_and_fills_in_painter_order
}
if [[ "${1:-}" == "--check" ]]; then
  # Regenera target/snapshots y lo compara con la baseline guardada.
  shots
  cd target/snapshots
  shasum -a 256 -c ../../snapshots/baseline/SHA256SUMS
  exit
fi
rm -rf target/snapshots/*.bmp
shots
rm -f snapshots/baseline/*.bmp
mkdir -p snapshots/baseline
cp target/snapshots/*.bmp snapshots/baseline/
(cd snapshots/baseline && shasum -a 256 *.bmp > SHA256SUMS)
echo "Baseline en snapshots/baseline ($(ls snapshots/baseline/*.bmp | wc -l | tr -d ' ') imágenes)"
