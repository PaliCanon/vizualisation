# 010 The map builds on ui-lib v0.3.0

status: open · rank: 3 · level: simple · type: build · request: the PaliCanon app's lockstep rule (app-001 v7, lock node; learn-pali's orchestrator, 2026-10-16) before the map's app fragment (011); the user's rule while GitHub Actions is blocked: "merge si les tests passent en local" · light path · after: [005, 006]

## Summary
Rank: difficulty 2 (the pin, the lock, the light theme's page colour), impact 3 (a visible change of the light theme; no behaviour) → 3.
The app (PaliCanon/app, Tauri) mounts Learn, Search and the Map built on one ui-lib tag: v0.3.0 (csr, csr::mount, the Store). One crate resolves one ui-lib, so the whole map moves first, as search-front 020 did (fc2e637: the pin only, no code change). ui-lib 0.2.0 changed the light palette to palicanon.app's cream: `--ps-page` #f8f6f0 → #fdf6e3, `--ps-gold` #9e7b22 → #84590a. The map's light canvas takes the page's colour again (#fdf6e3, so the canvas and the page stay one colour, the reason of 006's choice), its points keep the amber the user chose (#c9952e, variant B, 2026-10-16). The user sees the light theme before the merge.

## Log
| Step | Result | Output |
| --- | --- | --- |
| Define | done | orchestrator, 2026-10-16 |
