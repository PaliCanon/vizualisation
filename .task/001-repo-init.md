# 001 suttasmap joins the organisation as PaliCanon/vizualisation

status: in progress · level: standard · type: infra · request: user 2026-10-04 ("tu me ferais pas le tofu pour déployer sur viz.palicanon.app ?", "je pensais plus à une CI / CD") · process: ../../docs/prompts/repo-init.md

## Summary
The map moves from a local folder (no git) to a private repository in PaliCanon with the
organisation's CI/CD: push to main → dev (dev.viz.palicanon.app, Access), tag v* → prod
(viz.palicanon.app). Domain renamed map.palicanon.app → viz.palicanon.app in the code. Pages
deploy only (no GCP): the deploy identity (`github_repos`) is not needed.

## Log

| Step | Result | Output |
| --- | --- | --- |
| Define | done | name `vizualisation`, dev host `dev.viz.palicanon.app` + its certificate (user) |
| Local | done | git, CI (rankfusion jobs + site), deploy, infra, CLAUDE.md, README |
| GitHub | waiting | repo, secrets (org or repo), CODECOV_TOKEN, main protected |
| Infra | waiting | `tofu apply` (user) |
| Deploy | waiting | first green dev deploy, then a tag |
