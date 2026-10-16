# 010 The map builds on ui-lib v0.3.0

status: merged · rank: 3 · level: simple · type: build · request: the PaliCanon app's lockstep rule (app-001 v7, lock node; learn-pali's orchestrator, 2026-10-16) before the map's app fragment (011); the user's rule while GitHub Actions is blocked: "merge si les tests passent en local" · light path · after: [005, 006]

## Summary
Rank: difficulty 2 (the pin, the lock, the light theme's page colour), impact 3 (a visible change of the light theme; no behaviour) → 3.
The app (PaliCanon/app, Tauri) mounts Learn, Search and the Map built on one ui-lib tag: v0.3.0 (csr, csr::mount, the Store). One crate resolves one ui-lib, so the whole map moves first, as search-front 020 did (fc2e637: the pin only, no code change). ui-lib 0.2.0 changed the light palette to palicanon.app's cream: `--ps-page` #f8f6f0 → #fdf6e3, `--ps-gold` #9e7b22 → #84590a. The map's light canvas takes the page's colour again (#fdf6e3, so the canvas and the page stay one colour, the reason of 006's choice), its points keep the amber the user chose (#c9952e, variant B, 2026-10-16). The user sees the light theme before the merge.

## Log
| Step | Result | Output |
| --- | --- | --- |
| Define | done | orchestrator, 2026-10-16 |
| Build | done | 43c528c: app/Cargo.toml tag v0.3.0 (version "0.3"), Cargo.lock 0.3.0, the light canvas #fdf6e3 (ui-lib 0.2.0's cream page), points #c9952e kept; red first: the theme test expecting #fdf6e3 |
| Verify (local, CI's matrix and layout: ui-lib checked out at v0.3.0 beside) | done | fmt 0, leptosfmt 0, clippy app ssr 0 / hydrate wasm32 0 / map host 0 / map wasm32 0, test app 0 / map 0, rustdoc app 0 / map host 0 / map wasm32 0, msrv 1.88 0, deny 0, coverage 0, bundle 0, smoke 0, site 0, scripts 0, tofu 0 (after a provider download reset: unexpected EOF, retried) |
| Merge | done | squash 16087df (PR #9); released in v0.1.0, live at map.palicanon.app (light theme on the cream page) |

## Retro
One line (light path): the pin was trivial, the look was not — a ui-lib minor bump changed the light palette back to the very colours the map had just left; read the dependency's tokens before bumping, and show the user the look before the merge (it went out in v0.1.0 instead).
