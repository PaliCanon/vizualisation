# CLAUDE.md — vizualisation

@../docs/README.md
@../docs/conventions/index.md
@../claude-memory/MEMORY.md

This repository: the 3D map of the Pāli canon at viz.palicanon.app (React 19 + Tailwind v4 site,
`src/`; Rust → wasm32 map engine, `engine/`). Its data (`public/graph*.json`, `graph-sim.bin`,
`translations.json`, blurbs) is built by `scripts/` from SuttaCentral and from pali-embedder's
vectors, and committed: CI builds the site from it, never refetches it.

- Engine: build with the isolated toolchain (`~/.rustup-pc`, see ui-lab's README; the system Rust
  has no wasm32 target), `npm run build:engine` (cargo + wasm-bindgen 0.2.129, pinned in
  `engine/Cargo.toml`). Checks as CI: fmt, clippy `-D warnings` for the host and wasm32, tests,
  rustdoc, MSRV 1.88, cargo-deny, coverage.
- Site: `npm run dev` (port 5390), `npx tsc --noEmit -p .`, `npm run build`.
- Components copied from palisearch/web keep the source commit in their header: keep them in sync.
- Deploy: push to main → dev (dev.viz.palicanon.app, password: functions/_middleware.js, Pages secret
  DEV_PASSWORD on preview only, from the keychain: account palicanon, service viz-dev-password); tag v* → prod.
  Infra in `infra/`, applied from the laptop.
