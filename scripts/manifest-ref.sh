#!/usr/bin/env bash
# From search-front/scripts/manifest-ref.sh, unchanged but for this line and the
# example: here the `siblings` action reads the ref of ui-lib only (no api).
# The git ref (tag, rev or branch, in that order) a Cargo manifest names for
# a git dependency, on one line of the form `crate = { git = "...", branch = "..." }`.
#
#   scripts/manifest-ref.sh app/Cargo.toml palicanon-ui-lib   # → v0.1.1
#
# The `siblings` action checks ui-lib out at the ref the
# manifest names, so the sibling checkout the image is built from is the
# one Cargo would fetch. Fails (exit 1) when the crate is absent or names no ref.
set -euo pipefail

manifest=${1:?usage: manifest-ref.sh <Cargo.toml> <crate>}
crate=${2:?usage: manifest-ref.sh <Cargo.toml> <crate>}

line=$(grep -E "^${crate}[[:space:]]*=" "$manifest" | head -n 1 || true)
[ -n "$line" ] || { echo "manifest-ref: $crate is not in $manifest" >&2; exit 1; }
for key in tag rev branch; do
  value=$(printf '%s' "$line" | sed -nE "s/.*[{,][[:space:]]*${key}[[:space:]]*=[[:space:]]*\"([^\"]+)\".*/\1/p")
  if [ -n "$value" ]; then
    echo "$value"
    exit 0
  fi
done
echo "manifest-ref: $crate names no tag, rev or branch in $manifest" >&2
exit 1
