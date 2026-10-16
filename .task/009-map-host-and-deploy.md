# 009 The map at map.palicanon.app, deployed

status: design · rank: 4 · level: standard · type: infra · request: user, 2026-10-16 ("je préfère le nom map"; the showcase links to the map)

## Summary
Rank: difficulty 3 (a host rename in the Pages infra, the first real deploy of the Pages project, the showcase's link), impact 5 (the showcase's map scene and page point at a host that answers 404 today) → 4.
Today `viz.palicanon.app` resolves but the Pages project answers "Deployment Not Found": the map was never deployed. The user wants the name `map.palicanon.app` (dev: `dev.map.palicanon.app`). To do: rename the hosts in `infra/variables.tf` (prod/dev) and the README; the orchestrator applies (Pages domains: needs a Cloudflare token with Pages rights, see palisearch 049); a first deploy through the repository's workflow (read `.github/workflows/`), the data built or committed as the README says; the showcase already links to map.palicanon.app (www main). Remove the old viz.palicanon.app domain once map answers.

## Log
| Step | Result | Output |
| --- | --- | --- |
| Define | done | orchestrator, 2026-10-16 |
