# 008 (E) React, Vite and Node removed; Pages retired

status: open · rank: 6 · level: standard · type: refactor · request: user, 2026-10-12 ("remplace react par lepsos"), validated 2026-10-14 ("on fait la reecriture ! jaimerai bien que ce soit dans les guidelines des autres repo") · session: vizualisation-002-leptos (design dce546795076c4c v4, discussion d49d768dcc92341, process d4b32bf84ae3f40) · process: ../../docs/workflows/development-process.mmd · after: [006, 007]

## Summary
Rank: difficulty 4 (deletions, README, CLAUDE.md, CI), impact 6 (the old dev host and the Pages project go; prod switches) → 6.
Design: `002 design` v4 in the viewer (session vizualisation-002-leptos), validated by the user 2026-10-14. Decided by the user: Leptos SSR + hydrate on Cloud Run; ui-lib's AppShell around the map; the force layout ported to Rust; dev protected by a password inside the server; texts in Pāli on a first visit, then the interface language. Continuity (v4): the workspace, lints, ui-lib pin, i18n, rail, badge, CI and infra follow learn-pali and search-front (their files are the reference, quoted before diverging).
This task: remove `src/`, `index.html`, `package*.json`, `vite.config.ts`, `tsconfig*.json`, `functions/`, `scripts/*.mjs`, the old site job of CI, the Pages project and its domains in tofu, the `CLOUDFLARE_*` repo secrets; README and CLAUDE.md in English, describing the Rust workspace; `dev.viz.palicanon.app` retired after `viz.dev.palicanon.app` is checked.

## Lanes
Plan + Build by the lead (2026-10-22, user: «je pense quon peut virer le js»): every JS file goes but the Worker (`infra/worker/proxy.js`, `*.test.mjs`). One slice per commit, each with the docs it makes stale.

| Slice | Files | Behaviour / guard |
| --- | --- | --- |
| S1 React site | `src/`, `index.html`, `package*.json`, `vite.config.ts`, `tsconfig*`, `.github/actions/build`, ci.yml `site` job, check_workflows(_test).py docs, `.gitignore`, `Cargo.toml` profile comment, `scripts/image-hash.sh` + `app/tests/image_hash_script.rs` (non-input example), README/CLAUDE.md React lines | CI has no npm step; `build:engine` written as cargo + wasm-bindgen in the README |
| S2 Node data tools | `scripts/build-*.mjs`, `scripts/lib/`, `layout/tests/fixtures/gen.cjs`, `layout/src/params.rs`, `layout/tests/ngraph_parity.rs`, README "Data" | `small.json` kept, its generator named in git history; data tools in git history until 007 |
| S3 Pages function + guard | `functions/`, `app/src/gate.rs` doc, new `scripts/no_js_test.py` + its ci.yml step | test first (red on S1–S3's files): no tracked JS/TS/package file but the Worker's |
| S4 Strings | `scripts/port_strings.py` + test, ci.yml step, `app/src/pages/{content_*,types,mod}.rs` headers, `app/STRINGS.md`, CLAUDE.md | locales and content are ordinary source |
| S5 Pages infra | `infra/pages.tf`, `variables.tf`, `outputs.tf`, `versions.tf`, `cloudflare.tf`, `gcp.tf`, `prod.tfvars.example`, `infra/README.md`, deploy.yml lines 16–17 | `tofu fmt -check`, `tofu validate`; no plan/apply (orchestrator, with `-var-file=prod.tfvars`, A1) |
| S6 README + CLAUDE.md | both | the Rust workspace only, hosts, data committed, checks matrix |
| S7 public/ statics | the 22 React-only files of `public/` (ruling: orchestrator, 2026-10-22) | before/after GET of /favicon.ico, /og.png, /lotus.png, /robots.txt, /sitemap.xml, /apple-touch-icon.png, a /support/*.jpg on the release binary: same status and bytes |

Built in this order (S4 before S1: `port_strings_test.py` read `src/locales/` and failed once `src/` was gone): S4 4404d39, S1 ae141ef, S2 29dec38, S3 cf9308c, S5 af2cee1, S6 8f7617a, S7 5dfcb61, then 400e7c7 (README: the smoke test needs the Dockerfile's env). Checks from `<scratchpad>/008/lead/ws/vizualisation` (worktree moved there, ui-lib v0.3.1 as its own sibling: the shared ../ui-lib moved to 0.3.2 mid-gate; ruling of the orchestrator).

Assumptions: A3 the README's engine-alone commands build `engine/pkg` (probed: both exit 0). A1 prod is applied from a local `infra/prod.tfvars` (map.palicanon.app 200): the post-merge apply passes `-var-file=prod.tfvars`. A2 lowercase `pages` in `app/` is the app's footer pages, kept.

## Log
| Step | Result | Output |
| --- | --- | --- |
| Define | done | design v4 validated by the user 2026-10-14 |
| Plan | done | `## Lanes` S1–S7 (lead) |
| Build | done | S1–S7 in 8 commits (4404d39…400e7c7), 65 files removed; matrix on the tip all 0: fmt, leptosfmt, clippy ×6, tests ssr 324 / csr 190 / map 122 passed, rustdoc ×3, scripts (check_workflows 84, no_js 5, infra_probe 2, sh ×3, worker node 31), tofu fmt/init/validate; release build + smoke.sh 0; no tofu plan/apply |
| Review | done | reviewer: 008-review (d72322f9e20a247, sage #13774 #13868), 008-review-gemini (dd65aca4c3e4742, #13748 #13798): 9 findings, all ruled out on b9c3985 with the line that disproves each (process thread) |
| Verify (local, CI's matrix) | done | the lead's run on b9c3985 (fmt, leptosfmt, clippy ×6, tests ssr 324 / csr 190 / map 122, rustdoc ×3, scripts and the Worker's tests, check_workflows, tofu validate, release build + smoke: 0) plus msrv app 0, msrv map 0, deny 0, coverage 0, test-browser (csr, Chrome 155) 0 — ui-lib at v0.3.1 as CI |
| Merge | done | squash 3216826 (PR #12) |
| Infra (Pages) | done | user 2026-10-22: «cess de repondre», «si cest pas le cas supprime les variables cloudflare»: `tofu apply` of the targeted plan — 5 destroyed (palicanon-viz, domains and CNAMEs viz and dev.viz), 0 changed; viz and dev.viz stopped resolving at the authoritative NS ~20 s later; repo secrets CLOUDFLARE_ACCOUNT_ID and CLOUDFLARE_API_TOKEN deleted (no workflow read them; APP_ID, APP_PRIVATE_KEY remain). No deploy: 008 changes nothing the site serves |

