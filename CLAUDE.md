# CLAUDE.md — vizualisation

@../docs/README.md
@../docs/conventions/index.md
@../claude-memory/MEMORY.md

This repository: the 3D map of the Pāli canon at map.palicanon.app, in Rust only (design `002 design`
v4; the React site went in task 008). A Cargo workspace: `app/` (Leptos SSR + hydrate on ui-lib,
Cloud Run; also the csr Map fragment the PaliCanon app mounts), `engine/` (Rust →
wasm32 map renderer, generic), `layout/` (force layout, port of ngraph.forcelayout), `formats/` (the
data files' types, their only reader and writer), `i18n-check/` (the catalogue's placeholder check).
The data (`public/graph*.json`, `graph-sim.bin`, `translations.json`, `blurbs-*.json`) is committed:
CI and the image build from it, never refetch it. The Node tools that built it are in git history
(`7d2e46f`) until 007 ports them to Rust. The repository holds no JavaScript file
(`scripts/no_js_test.py` keeps it so).

- Toolchain: the isolated one (`~/.rustup-pc`, README "Run it"); the system Rust has no wasm32 target.
  Checks as CI (README "Checks"): fmt, leptosfmt, clippy `-D warnings` (app ssr, hydrate wasm32, csr
  wasm32 and host; the map crates host + wasm32), tests (app ssr and csr, map crates, the fragment in
  headless Chrome), rustdoc, MSRV 1.88, cargo-deny, coverage, `cargo leptos build --release`, the
  deploy's tests (Python, shell), tofu fmt/validate.
- App: `VIZ_GATE=off cargo leptos watch` (port 3301).
- Strings: `app/locales/*.json` (the `leptos_i18n` catalogue) and `app/src/pages/content_*.rs` (the
  footer pages) are ordinary source, edited by hand (their port from React: `app/STRINGS.md`).
- Deploy: push to main is CI only; tag v* → prod on Cloud Run (`vizualisation-prod`; public prod is
  served from the lsh-tech server). The dev environment is removed (task 022): no deployment uses
  `VIZ_GATE=password`. Infra in `infra/`, applied from the laptop with no `-var-file` (prod is the
  default; `infra/README.md`).
