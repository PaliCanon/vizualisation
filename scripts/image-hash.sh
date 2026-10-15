#!/usr/bin/env bash
# From search-front/scripts/image-hash.sh. What differs: one sibling (ui-lib;
# this service has no API client), and the workspace's other members
# (`engine`, `layout`, `formats`, which the Dockerfile copies) are inputs.
#
# The image tag (task 005, S5): `h-` and 16 hex characters of a SHA-256 over
# what shapes the image, so the same sources and the same sibling always give
# the same tag and an existing image is reused. Computed ONCE per CI run,
# after the sibling checkout, and handed to the build, the "exists?" check
# and the rollout.
#
#   scripts/image-hash.sh <ui-lib checkout>
#
# Inputs: the tracked files (git blob ids) of `app`, `engine`, `layout`,
# `formats`, `Cargo.toml`, `Cargo.lock`, `Dockerfile` and `scripts` of this
# repository (the Dockerfile text, so a base image or a tool version, and the
# smoke test are in the hash), and the HEAD commit of ui-lib. Not `infra/`,
# `.task/`, the docs or the React site (`src/`, `public/`): they do not change
# the image.
set -euo pipefail

if [ "$#" -ne 1 ]; then
  echo "usage: image-hash.sh <ui-lib checkout>" >&2
  exit 2
fi
ui_lib=$1
root=$(cd "$(dirname "$0")/.." && pwd)

head_of() { # head_of NAME DIR
  git -C "$2" rev-parse HEAD 2>/dev/null || { echo "image-hash: $1 ($2) is not a git checkout" >&2; return 1; }
}
ui_lib_head=$(head_of ui-lib "$ui_lib")

if command -v sha256sum >/dev/null 2>&1; then
  sha() { sha256sum; }
else
  sha() { shasum -a 256; }
fi

hash=$(
  {
    git -C "$root" ls-files -s app engine layout formats Cargo.toml Cargo.lock Dockerfile scripts
    echo "ui-lib $ui_lib_head"
  } | sha | cut -c1-16
)
echo "h-$hash"
