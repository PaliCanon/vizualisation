#!/bin/sh
# Fetch SuttaCentral's data into data/ (git-ignored). Sparse clones, only what the map uses.
# Idempotent: re-run to update.
#
#   data/sc-data      parallels, collection tree, acronyms, names, legacy HTML translations
#   data/bilara-data  Pāli root texts (to find each text's title segment) and the
#                     segmented translations (en, fr, vi)
set -e
cd "$(dirname "$0")/.."
mkdir -p data

sparse_clone() { # url dir paths...
  url=$1 dir=$2; shift 2
  [ -d "$dir" ] || git clone --depth 1 --filter=blob:none --sparse "$url" "$dir"
  git -C "$dir" sparse-checkout set --no-cone "$@"
  git -C "$dir" pull --depth 1 --ff-only
}

sparse_clone https://github.com/suttacentral/sc-data.git data/sc-data \
  '/relationship/' '/structure/' '/misc/' \
  '/sc_bilara_data/translation/en/sujato/name/' '/sc_bilara_data/root/*/*/name/' \
  '/html_text/en/pli/' '/html_text/fr/pli/' '/html_text/vi/pli/' \
  '/sc_bilara_data/root/en/blurb/' '/sc_bilara_data/translation/fr/blurb/' '/sc_bilara_data/_author.json'

sparse_clone https://github.com/suttacentral/bilara-data.git data/bilara-data \
  '/root/pli/ms/' '/translation/en/' '/translation/fr/' '/translation/vi/'

echo "data/ ready"
