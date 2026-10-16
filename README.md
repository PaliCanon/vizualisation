# vizualisation — Suttas Map

The 3D map of the Pāli canon at [map.palicanon.app](https://map.palicanon.app): SuttaCentral's
parallels, and a vectorial space built on the Pāli text.

**Being rewritten in Leptos** (design `002 design` v4, tasks 004–008 in `.task/`). Until task 008
the repository holds both:

- `app/`: the Leptos app (SSR + hydrate) on [`PaliCanon/ui-lib`](https://github.com/PaliCanon/ui-lib),
  served by Cloud Run behind a Cloudflare Worker — today its shell, routes, pages, gates and deploy
  (005); the map screen comes in 006;
- `src/`, `index.html`: the React site, still what `dev.viz.palicanon.app` serves (Cloudflare Pages)
  until 008 removes it.

Also: the 3D engine (`engine/`, Rust → WebAssembly, generic: it knows no text), the force layout
(`layout/`, a port of ngraph.forcelayout), the data files' types (`formats/`), the scripts that build
the data (`scripts/`) and the data (`public/`), the infra (`infra/`). **Never holds**: SuttaCentral's
clones (`data/`, made again by `npm run fetch-data`), pali-embedder's vectors (read from
`../pali-embedder` by `npm run build-sim`), any secret.

## Run it

```sh
# Toolchain with the wasm32 target, isolated from the system Rust:
export RUSTUP_HOME=$HOME/.rustup-pc/rustup CARGO_HOME=$HOME/.rustup-pc/cargo PATH=$HOME/.rustup-pc/cargo/bin:$PATH RUSTC_WRAPPER=sccache
# This service's own port: site 3301, live reload 4301 (Cargo.toml's leptos metadata).
VIZ_GATE=off cargo leptos watch
```

`VIZ_GATE` is required at every boot: `off` locally and in prod, `password` in dev (with
`DEV_PASSWORD`). A deployed build also requires `ORIGIN_SECRET` (the Worker's). A missing or blank
variable stops the service at boot, naming the variable, never its value. **No `.env` file.**

The app depends on `palicanon-ui-lib` by git tag (`v0.1.2`), with the workspace's `[patch]` to
`../ui-lib` (a worktree reaches it through the `wt/ui-lib` symlink).

The React site, until 008:

```sh
npm install
npm run build:engine        # the 3D engine (Rust -> wasm) -> engine/pkg
npm run dev                 # port 5390
```

## Data

Everything shown comes from **SuttaCentral** ([sc-data](https://github.com/suttacentral/sc-data),
[bilara-data](https://github.com/suttacentral/bilara-data)), except the positions of the vectorial
space, computed by [pali-embedder](https://github.com/Lsh0x/pali-embedder) on SuttaCentral's Pāli
text, with the *Digital Pāḷi Dictionary* bringing each word to its base form.

```sh
npm run fetch-data          # SuttaCentral: sc-data + bilara-data (partial clones in data/)
npm run build-graph         # SuttaCentral view   -> public/graph.json
npm run build-sim           # vectorial space     -> public/graph-sim.json + .bin
npm run build-translations  # translated texts    -> public/translations.json
```

| File | Holds | Source |
|---|---|---|
| `public/graph.json` | Pāli texts with a parallel, links by type, 3D positions (force layout) | sc-data `relationship/new_parallels.json` |
| `public/graph-sim.json`, `.bin` | The whole canon (7,286 texts), pairs of texts by similarity | pali-embedder: bilara-data `root/pli/ms` + DPD |
| `public/translations.json` | Texts translated into en / fr / vi on SuttaCentral, with their titles | bilara-data `translation/`, sc-data `html_text/*/pli` |

The vectors are read from `../pali-embedder/out/pali-lsa-kept-512.*` (Pāli LSA with formulas, rare
bigrams and WCCN). The data tools move to Rust in task 007.

## Checks

```sh
cargo fmt --all --check
leptosfmt --check app
cargo clippy -p vizualisation --features ssr --all-targets -- -D warnings
cargo clippy -p vizualisation --lib --features hydrate --target wasm32-unknown-unknown -- -D warnings
cargo clippy -p map-engine -p map-layout -p map-formats --all-targets -- -D warnings
cargo clippy -p map-engine --lib --target wasm32-unknown-unknown -- -D warnings
cargo test -p vizualisation --features ssr
cargo test -p map-engine -p map-layout -p map-formats
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps -p vizualisation --features ssr
cargo deny check advisories bans sources licenses
cargo leptos build
```

The same in CI (`.github/workflows/ci.yml`), plus `cargo check --locked` on Rust 1.88 (the MSRV),
coverage, the deploy's own tests and `tofu fmt`/`validate`:

```sh
python3 scripts/check_workflows_test.py && python3 scripts/check_workflows.py
python3 scripts/port_strings_test.py
bash scripts/manifest-ref.test.sh && bash scripts/serving-revision.test.sh
bash infra/scripts/origin-secret.test.sh
node --test 'infra/worker/*.test.mjs'
(cd infra && tofu fmt -check -recursive && tofu init -backend=false && tofu validate)
```

Functions stay under 50 lines (`clippy.toml`).

## Deploy

`main` deploys `map.dev.palicanon.app`, a `v*` tag `map.palicanon.app` (prod is added to `infra/`
once dev is checked, after 008; the React site keeps `viz.palicanon.app` on Pages until 008) — `.github/workflows/deploy.yml`,
from search-front's. Cloudflare (a Worker per host) is the only way in: the service answers 403 to
any request without the Worker's `x-origin-auth` secret. Dev asks for a password (`VIZ_GATE=password`,
`DEV_PASSWORD` from Secret Manager; the value in the keychain: account `palicanon`, service
`viz-dev-password`); prod runs the same image with the gate off and no access to that secret. The
image is built with no credential (`Dockerfile`): ui-lib is COPYed as a sibling checkout. CI's
checkouts use a GitHub App token (`.github/actions/siblings`); the repository secrets are `APP_ID`
and `APP_PRIVATE_KEY`, the variables `WIF_PROVIDER`, `DEPLOYER_SA` and `GCP_PROJECT`. The infra, the
Worker and the ordered first-time commands are in `infra/README.md` (OpenTofu, applied from the
laptop, state prefix `vizualisation`). The public-host check (`scripts/rollout-check.sh
--via-worker`) also loads `/`, `/sim/mn10` and `/privacy` in headless Chrome (`$CHROME`; on dev
through a launcher page that carries the password, never on Chrome's command line) and fails on a
hydration panic or a page without `<html data-hydrated>` (search-front 017, ui-lib 063).
