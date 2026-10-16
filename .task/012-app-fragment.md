# 012 The map runs inside the PaliCanon app — the site unchanged

status: open · rank: 7 · level: hard · type: feature · request: user, 2026-10-16 to learn-pali's orchestrator ("intègre-le dans la desktop app"), T9 of app-001 (workspace `app`, design v7) · session: vizualisation-012-app-fragment · after: [011]

## Summary
Rank: difficulty 7 (a third build, csr, beside ssr and hydrate; the first render computed in the browser; routes relative to a host), impact 6 (the site must not change; the app's critical path: its first public download waits for Learn, Search and the Map) → 7, hard: the main orchestrator runs it.
The pattern is fixed by two sister designs at consensus — learn-pali 048 v4 (session learn-pali-048-app-fragment, design d5caaeabfda6d43) and search-front 021 v4 (session search-front-021-app-fragment, design d15a1fc68949346) — mirrored here: a `csr` feature (leptos/csr + palicanon-ui-lib/csr v0.3.0, task 010) and a private `browser` feature (hydrate OR csr) that takes every browser-only path gated on hydrate today; `MapRoutes` (#[component(transparent)], relative routes the parent mounts: the site at /, the app in ParentRoute /map; links from use_matched()); `MapProvider` with a required data source (the site: the manifest's hashed URLs; the app: its bundled files); no AppShell, rail, footer or Title inside; the site's SSR document, routes and tests unchanged. The map's own seams (named to the app's orchestrator): `FirstRender` computed from the fetched data in csr (no server); the preferences (legend, texts language) through ui-lib 0.3.0's Store (cookies on the web, as today); unmount releases everything (EngineView's on_cleanup → stop) with a mount-unmount-mount browser test (the app wants no resource used while the map's tab is hidden).

## Log
| Step | Result | Output |
| --- | --- | --- |
| Define | in progress | orchestrator, 2026-10-16: design from the two sister designs |
