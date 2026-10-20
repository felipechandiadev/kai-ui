#!/usr/bin/env bash
# Recorta el mapa base de Protomaps a Parral y lo deja en
# assets/maps/parral.pmtiles (datos © OpenStreetMap contributors · Protomaps).
# Uso: scripts/extract-region.sh [AAAAMMDD]
#   AAAAMMDD: build diario de https://build.protomaps.com (por omisión, el de
#   hoy en UTC; Protomaps guarda solo los de los últimos días).
# Sin `pmtiles` en el PATH, descarga go-pmtiles en target/tools.
set -euo pipefail
cd "$(dirname "$0")/.."

PMTILES_VERSION=1.31.2
BUILD="${1:-$(date -u +%Y%m%d)}"
BBOX=-71.95,-36.25,-71.70,-36.05
MAXZOOM=15
OUT=assets/maps/parral.pmtiles

pmtiles_bin() {
  if command -v pmtiles >/dev/null; then
    command -v pmtiles
    return
  fi
  local dir=target/tools/go-pmtiles-$PMTILES_VERSION
  if [[ ! -x $dir/pmtiles ]]; then
    local os arch asset url
    os=$(uname -s)
    arch=$(uname -m)
    [[ $arch == aarch64 ]] && arch=arm64
    case $os in
      Darwin) asset=go-pmtiles-${PMTILES_VERSION}_Darwin_$arch.zip ;;
      Linux) asset=go-pmtiles_${PMTILES_VERSION}_Linux_$arch.tar.gz ;;
      *) echo "Sistema no soportado: $os" >&2; exit 1 ;;
    esac
    url=https://github.com/protomaps/go-pmtiles/releases/download/v$PMTILES_VERSION/$asset
    mkdir -p "$dir"
    curl -fsSL "$url" -o "$dir/$asset"
    case $asset in
      *.zip) unzip -qo "$dir/$asset" -d "$dir" ;;
      *) tar -xzf "$dir/$asset" -C "$dir" ;;
    esac
  fi
  echo "$dir/pmtiles"
}

PMTILES=$(pmtiles_bin)
mkdir -p "$(dirname "$OUT")"
"$PMTILES" extract "https://build.protomaps.com/$BUILD.pmtiles" "$OUT" \
  --bbox="$BBOX" --maxzoom="$MAXZOOM"
"$PMTILES" show "$OUT"
shasum -a 256 "$OUT"
ls -lh "$OUT"
