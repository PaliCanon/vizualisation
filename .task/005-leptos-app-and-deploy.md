# 005 (B) The Leptos app's skeleton, its gates and its Cloud Run deploy

status: open · rank: 9 · level: hard · type: feature · request: user, 2026-10-12 ("remplace react par lepsos"), validated 2026-10-14 ("on fait la reecriture ! jaimerai bien que ce soit dans les guidelines des autres repo") · session: vizualisation-002-leptos (design dce546795076c4c v4, discussion d49d768dcc92341, process d4b32bf84ae3f40) · process: ../../docs/workflows/development-process.mmd · after: [001]

## Summary
Rank: difficulty 9 (a new stack, a new deploy kind, infra in two clouds), impact 8 (security: two gates and a secret; production host) → 9, hard: the main orchestrator runs it.
Design: `002 design` v4 in the viewer (session vizualisation-002-leptos), validated by the user 2026-10-14. Decided by the user: Leptos SSR + hydrate on Cloud Run; ui-lib's AppShell around the map; the force layout ported to Rust; dev protected by a password inside the server; texts in Pāli on a first visit, then the interface language. Continuity (v4): the workspace, lints, ui-lib pin, i18n, rail, badge, CI and infra follow learn-pali and search-front (their files are the reference, quoted before diverging).
This task, from learn-pali's and search-front's files:
- `app/` crate (Leptos ssr + hydrate, cargo-leptos metadata as learn-pali), `palicanon-ui-lib` pinned by tag with the `[patch]` to `../ui-lib`; `leptos_i18n` 0.6.2 with `app/locales/{en,fr,vi}.json` ported from `src/locales`; AppShell with the rail Search · Learn · Map, Settings, footer, `EnvBadgeInfo`; the typed route table (`/`, `/sim`, `/pca`, `/umap`, `/tsne`, each with `/:uid`; pages acknowledgements, terms, privacy, legal, support; 404 noindex); SEO head, `robots.txt`, `sitemap.xml` (pages; texts in 006).
- The server: boot (`Config::from_env` + data load, any error → exit before binding), origin gate (`x-origin-auth`, 403), `/healthz` (behind the origin gate only), password gate (`VIZ_GATE` = password | off, required; HTTP Basic, constant time; fail closed).
- `Dockerfile`, `.github/workflows/{ci,deploy}.yml` and the siblings action (GitHub App token) as search-front; deploy main → dev `viz.dev.palicanon.app`, tag → prod; rollout no-traffic then checks (healthz 200, a hashed asset, 403 without the header, dev 401 without the password).
- `infra/` copied from search-front (envs map, dev first; state prefix `vizualisation`; runtime SA per env; secrets without versions; `scripts/origin-secret`; `worker/proxy.js` + tests in CI).
Needs from the user: the GitHub App's private key (repo secrets APP_ID, APP_PRIVATE_KEY), and the one-line addition of PaliCanon/vizualisation to `github_repos` in palisearch/infra (asked 2026-10-14).

## Log
| Step | Result | Output |
| --- | --- | --- |
| Define | done | design v4 validated by the user 2026-10-14 |
