# 009 The map at map.palicanon.app, deployed

status: dropped · rank: 4 · level: standard · type: infra · request: user, 2026-10-16 ("je préfère le nom map"; the showcase links to the map)

## Summary
Rank: difficulty 3 (a host rename in the Pages infra, the first real deploy of the Pages project, the showcase's link), impact 5 (the showcase's map scene and page point at a host that answers 404 today) → 4.
Today `viz.palicanon.app` resolves but the Pages project answers "Deployment Not Found": the map was never deployed. The user wants the name `map.palicanon.app` (dev: `map.dev.palicanon.app`). To do: rename the hosts in `infra/variables.tf` (prod/dev) and the README; the orchestrator applies (Pages domains: needs a Cloudflare token with Pages rights, see palisearch 049); a first deploy through the repository's workflow (read `.github/workflows/`), the data built or committed as the README says; the showcase already links to map.palicanon.app (www main). Remove the old viz.palicanon.app domain once map answers.

## Log
| Step | Result | Output |
| --- | --- | --- |
| Define | done | orchestrator, 2026-10-16 |

## Overlap (2026-10-16)
The vizualisation session (suttasmap-16) reports tasks 005/006 (feat/leptos-app, feat/map-screen, PRs #6/#7) already move the map to map.palicanon.app / map.dev.palicanon.app on Cloud Run behind a Worker, in Leptos (user: "pas de react, on fait en lepsos"). This task applies no Pages domain for map.* until the user rules which serves map.palicanon.app; if Leptos serves it directly, this task is dropped in favour of 005/006.
| Drop | done | user, 2026-10-16: "on devrait avoir la map avec lepsos" (and earlier: "pas de react, on fait en lepsos"; dev "devrait etre sur map.dev.palicanon.app"). map.palicanon.app is served by the Leptos map on Cloud Run behind a Worker, not the React map on Pages: the hosts are renamed in 005 (feat/leptos-app 548474d, PR #6; dev map.dev.palicanon.app, the <svc>.dev convention) and the deploy is 005's and 006's. Nothing of this task was applied. |

