# 007 (D) The data tools in Rust, stamped, checked in CI

status: deferred (until the data must be rebuilt; the plan is on branch feat/data-tools, 7f85b9f) · rank: 7 · level: standard · type: refactor · request: user, 2026-10-12 ("remplace react par lepsos"), validated 2026-10-14 ("on fait la reecriture ! jaimerai bien que ce soit dans les guidelines des autres repo") · session: vizualisation-002-leptos (design dce546795076c4c v4, discussion d49d768dcc92341, process d4b32bf84ae3f40) · process: ../../docs/workflows/development-process.mmd · after: [004]

## Summary
Rank: difficulty 6 (three Node scripts ported, the sim positions through `layout`), impact 7 (every data file; a wrong build ships wrong positions) → 7.
Design: `002 design` v4 in the viewer (session vizualisation-002-leptos), validated by the user 2026-10-14. Decided by the user: Leptos SSR + hydrate on Cloud Run; ui-lib's AppShell around the map; the force layout ported to Rust; dev protected by a password inside the server; texts in Pāli on a first visit, then the interface language. Continuity (v4): the workspace, lints, ui-lib pin, i18n, rail, badge, CI and infra follow learn-pali and search-front (their files are the reference, quoted before diverging).
This task: `tools/build-data` (Rust) replaces `scripts/build-graph.mjs`, `build-similarity.mjs`, `build-translations.mjs` and `lib/canon.mjs`, writing through `formats` and laying out through `layout`, every file stamped (SuttaCentral commits, vectors sha); `tools/reduce-3d.py` stays Python (offline, not in CI) and stamps its output; the data check (Rust test in CI): every file parses, the sim family shares one vectors sha and one node set, no uid equals a route segment. The output is compared with today's files (same nodes, same pairs) before replacing them.

## Log
| Step | Result | Output |
| --- | --- | --- |
| Define | done | design v4 validated by the user 2026-10-14 |
| Plan | done | on feat/data-tools 7f85b9f (slices S0–S7, process #13030 #13066 #13068): today's Node scripts reproduce the six public/ files byte for byte (the reference); bin, translations, blurbs can be byte-identical, nodes and pairs equal, positions cannot (median displacement 4.4 / 12.8). Rulings: vectors stamp over .bin and .json; REDUCE dropped (011 removed it); build-graph:all not ported. |
| Deferred | — | user 2026-10-22: «ce qui tourne en prod cest tres bien et sans le js n'est ce pas? Je pense quon peut virer le js» — 008 removes the JS now, data tools included; 007 resumes when the data must be rebuilt (the Node scripts stay in git history as the reference; open question Q1: keep the current arrangement by porting ngraph's initial placement, recommended, or a new one) |

