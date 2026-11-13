# 023 Images on GHCR, Cloud Run out of the infra

status: merged
rank: 4 — removal across infra, workflow, checker and docs; one new registry
level: simple (light path)
request: the user, 2026-11-13: « fini le search, quand tout est bon on vire cloudrun » and, for the registry,
« Passer à GHCR … Aller! »
repository: vizualisation

## Summary

Everything PaliCanon serves runs on the user's server ssh.lsh.tech (Lsh0x/lsh-tech, stacks/palicanon/prod),
containers pulled by digest behind a Cloudflare tunnel. The map's Cloud Run service gets no traffic any more.
This task removes it from the code and moves the image to GHCR.

## Design

- `deploy.yml` (tags `v*` only): one `image` job. It computes the same content hash (`scripts/image-hash.sh`),
  logs into ghcr.io with the job's `GITHUB_TOKEN` (`packages: write`), skips the build when
  `ghcr.io/palicanon/vizualisation:h-<hash>` exists (`docker buildx imagetools inspect`), otherwise builds and
  pushes it with the release tag and the `org.opencontainers.image.source` label; when it exists, only the release tag
  is added. The digest and tags go to `$GITHUB_STEP_SUMMARY`: the stack is updated from it by hand. No GCP step,
  no `id-token`.
- `check_workflows.py` and its tests: the Cloud Run rollout/rollback rules are replaced by GHCR rules.
  `release-values.sh` and `serving-revision.sh` (and their tests) are deleted: Cloud Run revision readers.
- `infra/`: every resource is removed (Cloud Run service, its public invoker binding, the runtime service account
  and its actAs binding, the origin secret and its two accessor bindings, the data source of `github-deployer`),
  with `cloudflare.tf` and the Cloudflare provider (state holds no Cloudflare resource). `versions.tf` keeps the
  gcs backend and the google provider, and `variables.tf` the two variables they need, for the one apply that
  destroys the old resources; `infra/README.md` says so and gives the order.
- Dead, left in place: the app's `K_SERVICE` / `ORIGIN_SECRET` origin guard and password gate (image-hash inputs)
  and `scripts/rollout-check.sh` with its Rust tests (no workflow calls it).

## Lanes

| lane | slice | files | model |
|---|---|---|---|
| A | deploy.yml and its checker | `.github/workflows/deploy.yml`, `scripts/check_workflows*.py`, deleted `scripts/release-values*`, `scripts/serving-revision*` | Sonnet |
| B | docs | `README.md`, `CLAUDE.md`, `infra/README.md` | Sonnet |
| lead | infra and CI | `infra/*.tf`, deleted `infra/scripts/*`, `scripts/infra_probe_test.py`, `ci.yml`, this file | — |

## Log

- 2026-11-13: Plan and Build. Lane A (workflow, checker: 90 tests, red 53 before) and lane B (docs) green; infra and CI by the lead.
  Checks: fmt, leptosfmt, clippy (ssr, csr host, hydrate/csr/map wasm32, map host on the native target:
  the linux target is not installed on this laptop), test ssr/csr/map (`--offline`: `--locked` fails on the ../ui-lib path
  patch; Cargo.lock not committed; two smoke tests flake under load, green alone), rustdoc, the Python and shell script tests,
  `tofu fmt -check`, `tofu validate` in a scratch copy. Not done by order: push, tag, plan, apply.
- Open: the user deletes the Cloud Run service `vizualisation-prod` with gcloud, then `tofu apply` in `infra/` (infra/README.md).

## Retro

Light path, one line: done from the shared GHCR brief; the first tag is the real test of the new image job.
