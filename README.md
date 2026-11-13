# vizualisation — Suttas Map

The 3D map of the Pāli canon at [map.palicanon.app](https://map.palicanon.app): SuttaCentral's
parallels, and a vectorial space built on the Pāli text. A Cargo workspace, in Rust only (design
`002 design` v4, tasks 004–008 in `.task/`; the React site it replaced was removed in task 008):

- `app/` (package `vizualisation`): the Leptos app (SSR + hydrate) on
  [`PaliCanon/ui-lib`](https://github.com/PaliCanon/ui-lib), served by Cloud Run behind a
  Cloudflare Worker — its shell, routes, footer pages, gates and deploy (005) and the map screen
  (006): each view at its path (`/` SuttaCentral, `/sim` the vectorial space), each text at
  `/<view>/<uid>` with its panel rendered by the server (a uid in no view is a 404), `?th=` the
  threshold, the browser's history the trail of texts visited; the legend and the texts' language
  kept in the `viz_legend` and `viz_textlang` cookies; `/sitemap.xml` lists every text once, at its
  first view. Also the Map fragment (012): the same map, client side only (`csr`), for the
  PaliCanon app, which mounts `MapRoutes` under its own `/map` with a `MapProvider` giving its
  bundled data files and no head (`app/src/fragment.rs`: what a host gives and needs); its
  preferences in `localStorage`, everything released when the app hides it;
- `engine/` (`map-engine`): the 3D engine, Rust → WebAssembly, generic (it knows no text);
- `layout/` (`map-layout`): the force layout, a port of ngraph.forcelayout 3.3.1;
- `formats/` (`map-formats`): the data files' types, their only reader and writer;
- `i18n-check/`: the placeholder-parity check of the app's catalogue, for `app/build.rs`.

Beside them: the data files (`public/`, committed), the deploy's scripts and their tests
(`scripts/`), the infra (`infra/`, OpenTofu, with the Worker `infra/worker/proxy.js`: the one
JavaScript file of the repository, kept so by `scripts/no_js_test.py`). **Never holds**: SuttaCentral's
clones (`data/`, made by `scripts/fetch-data.sh`), pali-embedder's vectors, any secret.

Comments that cite `src/…` (`src/App.tsx`, `src/locales/…`) or `scripts/*.mjs` name the React
site's files and the Node data tools, removed in task 008: they are in git history (last at
`7d2e46f`, e.g. `git show 7d2e46f:src/App.tsx`).

## Run it

```sh
# Toolchain with the wasm32 target, isolated from the system Rust:
export RUSTUP_HOME=$HOME/.rustup-pc/rustup CARGO_HOME=$HOME/.rustup-pc/cargo PATH=$HOME/.rustup-pc/cargo/bin:$PATH RUSTC_WRAPPER=sccache
# This service's own port: site 3301, live reload 4301 (Cargo.toml's leptos metadata).
VIZ_GATE=off cargo leptos watch
```

`VIZ_GATE` is required at every boot: `off` locally and in prod, `password` in dev (with
`DEV_PASSWORD`). Cloud Run (`K_SERVICE` present, even empty) also requires `ORIGIN_SECRET`
(the Worker's). On other hosts, including `LEPTOS_ENV=PROD` behind a private
Cloudflare tunnel, the origin secret is optional: a value enables the guard,
absence disables it, and a blank secret always refuses startup. `VIZ_GATE`
remains required on every host. Invalid configuration stops boot, naming the
variable, never its value. Set `VIZ_DEPLOYMENT_ENV=production` for production log metadata on the tunnel host. **No `.env` file.**

The app depends on `palicanon-ui-lib` by git tag (`v0.3.1`), with the workspace's `[patch]` to
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

The files are committed in `public/`; CI and the image build from them, never refetch them (the
Dockerfile copies them into the image, read once at boot):

| File | Holds | Source |
|---|---|---|
| `public/graph.json` | Pāli texts with a parallel, links by type, 3D positions (force layout) | sc-data `relationship/new_parallels.json` |
| `public/graph-sim.json`, `.bin` | The whole canon (7,286 texts), pairs of texts by similarity | pali-embedder: bilara-data `root/pli/ms` + DPD |
| `public/translations.json` | Texts translated into en / fr / vi on SuttaCentral, with their titles | bilara-data `translation/`, sc-data `html_text/*/pli` |
| `public/blurbs-en.json`, `blurbs-fr.json` | SuttaCentral's summary of each text, in en / fr | SuttaCentral |

The graph and translation files were built by Node scripts (`scripts/build-graph.mjs`,
`build-similarity.mjs`, `build-translations.mjs`; the vectors read from
`../pali-embedder/out/pali-lsa-kept-512.*`: Pāli LSA with formulas, rare bigrams and WCCN), removed
in task 008: they are in git history (`git show 7d2e46f:scripts/build-graph.mjs`) until task 007
ports them to Rust. `scripts/fetch-data.sh` still makes the SuttaCentral clones in `data/`.

## Operational logs

SSR emits JSON lines on stdout; hydration emits JSON to the local console.
No collector, exporter or backend is required and no browser log is sent over
the network. The CSR fragment leaves the subscriber to its host, which can opt
into `vizualisation::telemetry::init_browser()`; an existing subscriber is kept.

Every event carries `service.namespace: palicanon`, `service.name: map`,
`service.version` from the deployed `VIZ_ENV` tag/ref, `service.commit` from
`VIZ_SHA`, `deployment.environment.name`, `runtime`, `timestamp`, `severity`
and `event.name`. The release is the footer badge's and the browser reads the
same head metas. Deployment environment comes from `VIZ_DEPLOYMENT_ENV`
(letters, digits, `_` or `-`, at most 64 characters), otherwise Cloud Run's
`-dev` / `-prod` service suffix, or `development` locally. Missing browser
metas mean `unknown`; the deployed tag is independent of this environment.

Events cover lifecycle, data ready/load failures, admitted application HTTP
requests, origin/password refusals, browser data fetch/parse failures, engine
readiness/failures and panics. Asset labels and engine backends use closed lists.
URLs, directories, queries, text IDs, cookies, passwords, origin secrets, raw
parser/engine errors and panic payloads are excluded. Request IDs are generated
locally, not accepted from headers. HTTP duration ends at response headers.

INFO is the default; `VIZ_LOG_LEVEL=DEBUG` enables server detail, with dependency
targets still filtered out. Static file access and unknown paths are quiet;
health probes and HEAD summaries are DEBUG. Origin and password refusals each
emit at most one warning per minute per process. Successful browser data fetches
are DEBUG, browser logs otherwise stay at INFO. The rollout hydration gate
recognizes `runtime.panic` alongside legacy panic and wasm-trap messages.

## Checks

```sh
cargo fmt --all --check
leptosfmt --check app
cargo clippy -p vizualisation --features ssr --all-targets -- -D warnings
cargo clippy -p vizualisation --lib --features hydrate --target wasm32-unknown-unknown -- -D warnings
cargo clippy -p vizualisation --no-default-features --features csr --target wasm32-unknown-unknown --lib --test csr -- -D warnings
cargo clippy -p vizualisation --no-default-features --features csr --all-targets -- -D warnings
cargo clippy -p map-engine -p map-layout -p map-formats --all-targets -- -D warnings
cargo clippy -p map-engine -p map-layout -p map-formats --lib --target wasm32-unknown-unknown -- -D warnings
cargo test -p vizualisation --features ssr
cargo test -p vizualisation --no-default-features --features csr
cargo test -p map-engine -p map-layout -p map-formats
# The fragment in a headless Chrome (WebGL2 on SwiftShader, app/webdriver.json); chromedriver
# matching the local Chrome, wasm-bindgen-test-runner matching Cargo.lock's wasm-bindgen:
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner CHROMEDRIVER=<chromedriver> \
  WASM_BINDGEN_TEST_TIMEOUT=900 cargo test -p vizualisation --no-default-features --features csr \
  --target wasm32-unknown-unknown --test csr
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps -p vizualisation --features ssr
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps -p map-engine -p map-layout -p map-formats
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps -p map-engine -p map-layout -p map-formats --target wasm32-unknown-unknown
cargo deny check advisories bans sources licenses
cargo leptos build --release
# The smoke test as the image runs it (the Dockerfile's env: hashed file names, the data dir):
LEPTOS_OUTPUT_NAME=vizualisation LEPTOS_SITE_ROOT=target/site LEPTOS_SITE_PKG_DIR=pkg \
  LEPTOS_HASH_FILES=true VIZ_DATA_DIR=public scripts/smoke.sh target/release/vizualisation
```

The same in CI (`.github/workflows/ci.yml`, one job per group), plus `cargo check --locked` on Rust
1.88 (the MSRV), coverage (`cargo llvm-cov`), the deploy's own tests and `tofu fmt`/`validate`:

```sh
python3 scripts/check_workflows_test.py && python3 scripts/check_workflows.py
python3 scripts/infra_probe_test.py
python3 scripts/no_js_test.py
bash scripts/manifest-ref.test.sh && bash scripts/serving-revision.test.sh
bash infra/scripts/origin-secret.test.sh
bash scripts/release-values.test.sh && bash infra/scripts/lifecycle.test.sh
node --test 'infra/worker/*.test.mjs'
(cd infra && tofu fmt -check -recursive && tofu init -backend=false && tofu validate)
```

| CI job | What |
| --- | --- |
| `fmt` | rustfmt, leptosfmt |
| `clippy` | app ssr, hydrate wasm32, csr wasm32, csr host; map crates host, wasm32 |
| `test`, `test-browser` | app ssr and csr, map crates; the fragment in headless Chrome |
| `docs` | rustdoc `-D warnings`: app ssr, map crates host and wasm32 |
| `msrv`, `deny`, `coverage` | Rust 1.88, cargo-deny, llvm-cov |
| `bundle` | `cargo leptos build --release`, as the Dockerfile |
| `scripts` | the Python, shell and Worker (`node --test`) tests above |
| `tofu` | `fmt -check`, `validate` |

Functions stay under 50 lines (`clippy.toml`).

## Deploy

`main` deploys `map.dev.palicanon.app`, a `v*` tag `map.palicanon.app` — `.github/workflows/deploy.yml`,
from search-front's. Cloudflare (a Worker per host) is the only way in: the service answers 403 to
any request without the Worker's `x-origin-auth` secret. Dev asks for a password (`VIZ_GATE=password`,
`DEV_PASSWORD` from Secret Manager; the value in the keychain: account `palicanon`, service
`viz-dev-password`); prod runs the same image with the gate off and no access to that secret. The
image is built with no credential (`Dockerfile`): ui-lib is COPYed as a sibling checkout. CI's
checkouts use a GitHub App token (`.github/actions/siblings`); the repository secrets are `APP_ID`
and `APP_PRIVATE_KEY`, the variables `WIF_PROVIDER`, `DEPLOYER_SA` and `GCP_PROJECT`. The infra, the
Worker and the ordered first-time commands are in `infra/README.md` (OpenTofu, applied from the
laptop, state prefix `vizualisation`, every apply with `-var-file=prod.tfvars`). The public-host
check (`scripts/rollout-check.sh --via-worker`) also loads `/`, `/sim/mn10` and `/privacy` in
headless Chrome (`$CHROME`; on dev through a launcher page that carries the password, never on
Chrome's command line) and fails on a hydration panic or a page without `<html data-hydrated>`
(search-front 017, ui-lib 063).

The footer badge is the deploy's, not the image's: `deploy.yml` writes `VIZ_ENV` (the tag `v0.1.8`, or `main`) and `VIZ_SHA` (empty for a tag, else 7 characters) on the Cloud Run revision with `gcloud run deploy --update-env-vars` (never `--set-env-vars`, which would wipe `RUST_LOG`, `VIZ_GATE` and the `ORIGIN_SECRET` reference); the server reads them at start (absent: `dev`), writes two `<meta name="viz-release-...">` and the browser reads them back; `/health` says `ok <env>[ <sha>]`; `scripts/rollout-check.sh` asserts the metas before the traffic moves (exit 8). Tofu ignores the container env: `tofu apply` it before the first deploy that writes the env (`infra/README.md`). The Map fragment of the PaliCanon app has no footer, so no badge.
