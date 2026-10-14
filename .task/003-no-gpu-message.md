# 003 The map says so when the browser has neither WebGPU nor WebGL2

status: merged · rank: 2 · level: simple · type: feature · request: user, 2026-10-14 ("on a la possibilite de verifier que lutilisateur est webgpu? sinon afficher un message ?") · light path · after: [002]

## Summary
Rank: difficulty 2 (App, the engine adapter, three catalogues), impact 2 (only browsers that could not draw the map see it) → 2.
Decided by the orchestrator: the message shows only when neither WebGPU nor WebGL2 starts (WebGL2 draws the same map, a warning there would be noise). `<html data-engine>` names the API that draws (`BrowserWebGpu` / `Gl`). Ruled out after the check: the sentence "the search and the texts still work" — the text panel needs the engine today (008's Leptos panel, server-rendered, will not).

## Log
| Step | Result | Output |
| --- | --- | --- |
| Build | done | tsc 0, build ok |
| Check | done | headless Chrome: normal → data-engine=BrowserWebGpu, no message; navigator.gpu and WebGL contexts removed (script injected in index.html, restored) → the message, screenshot read |
| Merge | done | PR #3 squash 073bce3, CI 10/10 green |

## Retro
One line (light path): same as 002 — no task file before the first commit; and a message's promise is checked against the code before it ships (the removed sentence).
