# 011 The map keeps two views: SuttaCentral and the vectorial space

status: open · rank: 4 · level: standard · type: refactor · request: user, 2026-10-16 ("dans la map jaimerai quon retire les visu test. on garde sutta central et le vector space comme il est.") · light path · after: [010]

## Summary
Rank: difficulty 4 (the views are named in the route table, the data load, the manifest, the menu, the strings, the image, the tests), impact 4 (visible: the view menu and three addresses go; before the first prod tag) → 4.
The three test views (PCA, UMAP, t-SNE: `graph-sim-{pca,umap,tsne}.json`, made by `scripts/reduce-3d.py`) are removed from the Leptos map and its data: their routes become 404 (they were never public), the view menu holds SuttaCentral and the vectorial space, the server no longer loads or serves their files, the image no longer carries them, their strings go. SuttaCentral and the vectorial space stay as they are. The React site (until 008) drops them too so it still builds. Done before the first prod tag (v0.1.0).

## Log
| Step | Result | Output |
| --- | --- | --- |
| Define | done | orchestrator, 2026-10-16 |
