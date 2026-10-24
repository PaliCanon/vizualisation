# 018 The map on a phone: the canvas owns the touch gestures

status: merged · rank: 2 · level: simple · type: fix · request: the user, 2026-10-24 (« On a quelques soucis sur la map sur tel aussi, se déplacer, zoom etc. c'est pas fluide »)
worktree: /Users/lsh/projects/palicanon/wt/vizualisation-018/vizualisation (branch `fix/map-touch-action`), paired `wt/vizualisation-018/ui-lib` at v0.4.3

## Summary
Rank: difficulty 1 (one class on the canvas), impact 2 (every touch on the map). Light path, no agents (user, 2026-10-22), no voice at rank 2; the user checks on the phone, on dev.

The map's canvas had no `touch-action`: the browser kept its own gesture handling (page pan, pinch zoom, pull to refresh) under the pointer events, so a drag or a pinch on a phone competes with the page, stutters, or ends in a `pointercancel`. `touch-none` on the canvas gives it every touch gesture; the engine's pointer handling (one pointer rotates, two pinch, capture on press) is unchanged. Read in the code while looking: the device pixel ratio is already capped at 2 (`view.rs`), the camera's rotate has a damping of 0.05 (orbit feel, by design), a pinch is turned into wheel steps without an anchor on the fingers. Those are the next candidates if the phone still feels slow after this one: the user says which (a laggy drag, a jumpy zoom, the page moving).

## Log
| Step | Result | Output |
| --- | --- | --- |
| Build | ✓ | `touch-none` on the canvas, the SSR test updated |
| Matrix | ✓ | CI's commands: fmt, leptosfmt, clippy ssr/hydrate, test ssr and csr (native), doc; the browser tests in CI |
| Check | ✓ | the user, 2026-10-24, on the phone on map.palicanon.app (v0.2.5): « map test niquel » |

## Retro

## Merge
060755d on main (squash), 2026-10-24; prod v0.2.5.

## Retro
No agents: one line. The merge preceded the user's check because dev deploys from main only and the user had no access to dev from the phone: prod was the check. Lesson: a phone check needs prod or a dev host the phone reaches.
