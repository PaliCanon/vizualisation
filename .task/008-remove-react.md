# 008 (E) React, Vite and Node removed; Pages retired

status: open · rank: 6 · level: standard · type: refactor · request: user, 2026-10-12 ("remplace react par lepsos"), validated 2026-10-14 ("on fait la reecriture ! jaimerai bien que ce soit dans les guidelines des autres repo") · session: vizualisation-002-leptos (design dce546795076c4c v4, discussion d49d768dcc92341, process d4b32bf84ae3f40) · process: ../../docs/workflows/development-process.mmd · after: [006, 007]

## Summary
Rank: difficulty 4 (deletions, README, CLAUDE.md, CI), impact 6 (the old dev host and the Pages project go; prod switches) → 6.
Design: `002 design` v4 in the viewer (session vizualisation-002-leptos), validated by the user 2026-10-14. Decided by the user: Leptos SSR + hydrate on Cloud Run; ui-lib's AppShell around the map; the force layout ported to Rust; dev protected by a password inside the server; texts in Pāli on a first visit, then the interface language. Continuity (v4): the workspace, lints, ui-lib pin, i18n, rail, badge, CI and infra follow learn-pali and search-front (their files are the reference, quoted before diverging).
This task: remove `src/`, `index.html`, `package*.json`, `vite.config.ts`, `tsconfig*.json`, `functions/`, `scripts/*.mjs`, the old site job of CI, the Pages project and its domains in tofu, the `CLOUDFLARE_*` repo secrets; README and CLAUDE.md in English, describing the Rust workspace; `dev.viz.palicanon.app` retired after `viz.dev.palicanon.app` is checked.

## Log
| Step | Result | Output |
| --- | --- | --- |
| Define | done | design v4 validated by the user 2026-10-14 |
