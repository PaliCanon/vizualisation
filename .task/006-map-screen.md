# 006 (C) The map screen in Leptos

status: open · rank: 9 · level: hard · type: feature · request: user, 2026-10-12 ("remplace react par lepsos"), validated 2026-10-14 ("on fait la reecriture ! jaimerai bien que ce soit dans les guidelines des autres repo") · session: vizualisation-002-leptos (design dce546795076c4c v4, discussion d49d768dcc92341, process d4b32bf84ae3f40) · process: ../../docs/workflows/development-process.mmd · after: [004, 005]

## Summary
Rank: difficulty 9 (the whole of today's App.tsx and its components, the engine bound from Rust, SSR of the panel), impact 7 (every user-facing behaviour of the map) → 9, hard.
Design: `002 design` v4 in the viewer (session vizualisation-002-leptos), validated by the user 2026-10-14. Decided by the user: Leptos SSR + hydrate on Cloud Run; ui-lib's AppShell around the map; the force layout ported to Rust; dev protected by a password inside the server; texts in Pāli on a first visit, then the interface language. Continuity (v4): the workspace, lints, ui-lib pin, i18n, rail, badge, CI and infra follow learn-pali and search-front (their files are the reference, quoted before diverging).
This task: `EngineView` (hydrate only, engine called from Rust, no JS layer), the views and threshold, the legend (cookie `viz_legend`), search, the text panel (server-rendered at `/:view/:uid`), `open_text(uid, view)` and `texts_lang(...)` as pure tested functions, the texts-language chip (cookie `viz_textlang`), tooltip, keys, wheel, pinch, mobile drawer, the no-GPU message (003), the sitemap's text entries (each text once, at its canonical view). Parity list: the discussion thread's first message.

## Log
| Step | Result | Output |
| --- | --- | --- |
| Define | done | design v4 validated by the user 2026-10-14 |
