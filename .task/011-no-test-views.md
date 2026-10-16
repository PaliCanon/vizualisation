# 011 The map keeps two views: SuttaCentral and the vectorial space

status: merged · rank: 4 · level: standard · type: refactor · request: user, 2026-10-16 ("dans la map jaimerai quon retire les visu test. on garde sutta central et le vector space comme il est.") · light path · after: [010]

## Summary
Rank: difficulty 4 (the views are named in the route table, the data load, the manifest, the menu, the strings, the image, the tests), impact 4 (visible: the view menu and three addresses go; before the first prod tag) → 4.
The three test views (PCA, UMAP, t-SNE: `graph-sim-{pca,umap,tsne}.json`, made by `scripts/reduce-3d.py`) are removed from the Leptos map and its data: their routes become 404 (they were never public), the view menu holds SuttaCentral and the vectorial space, the server no longer loads or serves their files, the image no longer carries them, their strings go. SuttaCentral and the vectorial space stay as they are. The React site (until 008) drops them too so it still builds. Done before the first prod tag (v0.1.0).

## Log
| Step | Result | Output |
| --- | --- | --- |
| Define | done | orchestrator, 2026-10-16 |
| Build (lane) | done | 2c051ee: the routes /pca, /umap, /tsne (and /…/:uid) are the 404; the menu, the data load (6 files), the image, the strings, formats' sim family, the React MAPS: two views; public/graph-sim-{pca,umap,tsne}.json and scripts/reduce-3d.py deleted; red first: the route tests (/pca 200 → 404) |
| Verify (local, CI's matrix and layout: ui-lib v0.3.0 beside) | done | fmt 0, leptosfmt 0, clippy app ssr 0 / hydrate wasm32 0 / map host 0 / map wasm32 0, test app 0 / map 0, rustdoc app 0 / map host 0 / map wasm32 0, msrv 1.88 0, deny 0, coverage 0, bundle 0, smoke 0, site 0, scripts 0, tofu 0 (after network resets on the provider download, retried) |
| Merge | done | squash 223e952 (PR #10), after main was merged into the branch keeping its tree (the 010 log restored from main); released in v0.1.0 |

## Retro
One line (light path): a branch cut from an unsquashed branch conflicts with its own squash on main; merging main with `-X ours` kept the branch's stale task file (010's log lost, restored by hand) — after such a merge, diff the tree against the validated one and take main's task files.
