# CLAUDE.md — vizualisation

@../docs/README.md
@../docs/conventions/index.md
@../claude-memory/MEMORY.md

This repository: the 3D map of the Pāli canon at map.palicanon.app, being rewritten from React to
Leptos (design `002 design` v4, tasks 004–008 in `.task/`). A Cargo workspace: `app/` (Leptos SSR +
hydrate on ui-lib, Cloud Run behind a Worker), `engine/` (Rust → wasm32 map renderer, generic),
`layout/` (force layout, port of ngraph.forcelayout), `formats/` (the data files' types, their only
reader and writer). The React site (`src/`) stays until 008. Its data (`public/graph*.json`,
`graph-sim.bin`, `translations.json`, blurbs) is built by `scripts/` from SuttaCentral and from
pali-embedder's vectors, and committed: CI builds from it, never refetches it.

- Toolchain: the isolated one (`~/.rustup-pc`, README "Run it"); the system Rust has no wasm32 target.
  Checks as CI (README "Checks"): fmt, leptosfmt, clippy `-D warnings` (app ssr + hydrate wasm32,
  the map crates host + wasm32), tests, rustdoc, MSRV 1.88, cargo-deny, coverage, the deploy's tests,
  tofu validate.
- App: `VIZ_GATE=off cargo leptos watch` (port 3301). React site until 008: `npm run dev` (5390).
- Strings: `app/locales/*.json` (the `leptos_i18n` catalogue) and `app/src/pages/content_*.rs` (the
  footer pages) are ordinary source, edited by hand (their port from React: `app/STRINGS.md`).
- Deploy: push to main → dev (map.dev.palicanon.app on Cloud Run, password: `VIZ_GATE=password`,
  `DEV_PASSWORD` from Secret Manager, value in the keychain: account palicanon, service
  viz-dev-password); tag v* → prod. dev.viz.palicanon.app (Pages, React) until 008. Infra in
  `infra/`, applied from the laptop (`infra/README.md`).
