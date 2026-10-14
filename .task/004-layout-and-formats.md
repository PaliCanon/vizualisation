# 004 (A) The force layout and the data formats as Rust crates

status: open · rank: 7 · level: standard · type: feature · request: user, 2026-10-12 ("remplace react par lepsos"), validated 2026-10-14 ("on fait la reecriture ! jaimerai bien que ce soit dans les guidelines des autres repo") · session: vizualisation-002-leptos (design dce546795076c4c v4, discussion d49d768dcc92341, process d4b32bf84ae3f40) · process: ../../docs/workflows/development-process.mmd · after: [001]

## Summary
Rank: difficulty 7 (a port of ngraph.forcelayout with matching behaviour, a new crate of types every later task reads), impact 6 (the data contract of 005–007; no production change yet) → 7.
Design: `002 design` v4 in the viewer (session vizualisation-002-leptos), validated by the user 2026-10-14. Decided by the user: Leptos SSR + hydrate on Cloud Run; ui-lib's AppShell around the map; the force layout ported to Rust; dev protected by a password inside the server; texts in Pāli on a first visit, then the interface language. Continuity (v4): the workspace, lints, ui-lib pin, i18n, rail, badge, CI and infra follow learn-pali and search-front (their files are the reference, quoted before diverging).
This task: two crates in the workspace (as learn-pali's members), each generic where the design says so:
- `layout`: the force layout (port of ngraph.forcelayout 3.3 as used in `src/map/rust-engine.ts`: same forces and parameters, deterministic seed), no corpus; used live by `engine` (wasm32) and by the data tools (007). Tests: a seeded run is reproducible; a two-cluster graph separates; the energy falls.
- `formats`: the types of every data file (`graph.json`, `graph-sim.json`, `graph-sim.bin`, `graph-sim-{pca,umap,tsne}.json`, `translations.json`, `blurbs-*.json`) + the stamp (source commits, vectors sha); the only reader (app) and writer (tools). Tests: every committed file in `public/` parses (the data check of 007 grows from this).
- The workspace gets learn-pali's lints (`unsafe_code = "forbid"`, `missing_docs`, clippy `pedantic`), and `engine` is brought under them (11 warnings on the host today).
Ruled out here: the app (005), the map screen (006), the tools (007).

## Log
| Step | Result | Output |
| --- | --- | --- |
| Define | done | design v4 validated by the user 2026-10-14 |
