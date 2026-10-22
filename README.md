# vizualisation — Suttas Map

The 3D map of the Pāli canon at [map.palicanon.app](https://map.palicanon.app): SuttaCentral's
parallels, and a vectorial space built on the Pāli text.

**Being rewritten in Leptos** (design `002 design` v4, tasks 004–008 in `.task/`). Until task 008
the repository holds both:

- `app/`: the Leptos app (SSR + hydrate) on [`PaliCanon/ui-lib`](https://github.com/PaliCanon/ui-lib),
  served by Cloud Run behind a Cloudflare Worker — its shell, routes, pages, gates and deploy (005)
  and the map screen (006): each view at its path (`/` SuttaCentral, `/sim` the vectorial space), each
  text at `/<view>/<uid>` with its panel rendered by the server (a uid in no view is a 404),
  `?th=` the threshold, the browser's history the trail of texts visited; the legend and the
  texts' language kept in the `viz_legend` and `viz_textlang` cookies; `/sitemap.xml` lists
  every text once, at its first view;
- the Map fragment (012): the same map, client side only (`csr`), for the PaliCanon app, which
  mounts `MapRoutes` under its own `/map` with a `MapProvider` giving its bundled data files and
  no head (`app/src/fragment.rs`: what a host gives and needs); its preferences in
  `localStorage`, everything released when the app hides it.

Also: the 3D engine (`engine/`, Rust → WebAssembly, generic: it knows no text), the force layout
(`layout/`, a port of ngraph.forcelayout), the data files' types (`formats/`), the deploy's scripts
(`scripts/`) and the data (`public/`), the infra (`infra/`). **Never holds**: SuttaCentral's
clones (`data/`, made by `scripts/fetch-data.sh`), pali-embedder's vectors, any secret.

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

The app depends on `palicanon-ui-lib` by git tag (`v0.3.0`), with the workspace's `[patch]` to
`../ui-lib` (a worktree reaches it through the `wt/ui-lib` symlink).

### The engine alone

The app builds the engine into its own bundle. To build it alone, as a wasm-bindgen package in
`engine/pkg` (git-ignored), with `wasm-bindgen-cli` at Cargo.lock's version:

```sh
cargo build -p map-engine --target wasm32-unknown-unknown --profile wasm-release
wasm-bindgen --target web --out-dir engine/pkg target/wasm32-unknown-unknown/wasm-release/map_engine.wasm
```

## Data

Everything shown comes from **SuttaCentral** ([sc-data](https://github.com/suttacentral/sc-data),
[bilara-data](https://github.com/suttacentral/bilara-data)), except the positions of the vectorial
space, computed by [pali-embedder](https://github.com/Lsh0x/pali-embedder) on SuttaCentral's Pāli
text, with the *Digital Pāḷi Dictionary* bringing each word to its base form.

The files are committed; the Node scripts that built them (`scripts/build-graph.mjs`,
`build-similarity.mjs`, `build-translations.mjs`) were removed in task 008 and are in git history
(`7d2e46f`) until task 007 ports them to Rust. `scripts/fetch-data.sh` still makes the SuttaCentral
clones in `data/`.

| File | Holds | Source |
|---|---|---|
| `public/graph.json` | Pāli texts with a parallel, links by type, 3D positions (force layout) | sc-data `relationship/new_parallels.json` |
| `public/graph-sim.json`, `.bin` | The whole canon (7,286 texts), pairs of texts by similarity | pali-embedder: bilara-data `root/pli/ms` + DPD |
| `public/translations.json` | Texts translated into en / fr / vi on SuttaCentral, with their titles | bilara-data `translation/`, sc-data `html_text/*/pli` |

The vectors were read from `../pali-embedder/out/pali-lsa-kept-512.*` (Pāli LSA with formulas, rare
bigrams and WCCN).

## Checks

```sh
cargo fmt --all --check
leptosfmt --check app
cargo clippy -p vizualisation --features ssr --all-targets -- -D warnings
cargo clippy -p vizualisation --lib --features hydrate --target wasm32-unknown-unknown -- -D warnings
cargo clippy -p vizualisation --no-default-features --features csr --target wasm32-unknown-unknown --lib --test csr -- -D warnings
cargo clippy -p map-engine -p map-layout -p map-formats --all-targets -- -D warnings
cargo clippy -p map-engine --lib --target wasm32-unknown-unknown -- -D warnings
cargo test -p vizualisation --features ssr
cargo test -p map-engine -p map-layout -p map-formats
# The fragment in a headless Chrome (WebGL2 on SwiftShader, app/webdriver.json); chromedriver
# matching the local Chrome, wasm-bindgen-test-runner matching Cargo.lock's wasm-bindgen:
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner CHROMEDRIVER=<chromedriver> \
  WASM_BINDGEN_TEST_TIMEOUT=900 cargo test -p vizualisation --no-default-features --features csr \
  --target wasm32-unknown-unknown --test csr
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps -p vizualisation --features ssr
cargo deny check advisories bans sources licenses
cargo leptos build
```

The same in CI (`.github/workflows/ci.yml`), plus `cargo check --locked` on Rust 1.88 (the MSRV),
coverage, the deploy's own tests and `tofu fmt`/`validate`:

```sh
python3 scripts/check_workflows_test.py && python3 scripts/check_workflows.py
python3 scripts/no_js_test.py
bash scripts/manifest-ref.test.sh && bash scripts/serving-revision.test.sh
bash infra/scripts/origin-secret.test.sh
node --test 'infra/worker/*.test.mjs'
(cd infra && tofu fmt -check -recursive && tofu init -backend=false && tofu validate)
```

Functions stay under 50 lines (`clippy.toml`).

## Deploy

`main` deploys `map.dev.palicanon.app`, a `v*` tag `map.palicanon.app` (prod is added to `infra/`
once dev is checked, after 008) — `.github/workflows/deploy.yml`,
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
