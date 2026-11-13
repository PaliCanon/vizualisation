# 022 Drop the dev environment

status: merged
rank: 4 — removal across workflows, infra and docs; no new behaviour
level: simple (light path)
request: the user, 2026-11-13: « vire le dev totalement »
repository: vizualisation

## Summary

PaliCanon prod now runs on the user's server ssh.lsh.tech (Lsh0x/lsh-tech,
stacks/palicanon/prod, images by digest). The dev environment of the map
(Cloud Run `vizualisation-dev`, its Worker and Custom Domain
`map.dev.palicanon.app`, its secrets) is removed. Prod on Cloud Run is NOT
touched here (removed later, separately): every prod resource keeps its
address and arguments, and the tag path (`v*` → image build, rollout) is
unchanged.

## Design

- `deploy.yml`: triggered by `v*` tags only; a push to main is CI only. The
  dev password steps, dev host and dev env selection are gone.
- `infra/`: `envs` holds `prod` only (default); the dev password secret, its
  IAM members, the Worker script and Custom Domain (and the zone data source,
  `zone` and `cloudflare_account_id` variables) are removed. The cloudflare
  provider block stays for this apply: state still holds the dev Worker
  resources and the provider is needed to destroy them.
- The Worker code and tests (`infra/worker/`) are removed: no Worker is left.
- Not touched: `app/`, `Dockerfile`, `scripts/` (except tests that assert
  deploy.yml's dev branches): they are inputs of the image hash.

## Lanes

| lane | slice | files | model |
|---|---|---|---|
| A | workflows and their tests | `.github/workflows/deploy.yml`, `scripts/check_workflows_test.py`, `scripts/release-values.test.sh` | Sonnet |
| B | docs and Worker removal | `README.md`, `CLAUDE.md`, `infra/README.md`, `infra/worker/`, `infra/scripts/` | Sonnet |
| lead | tofu | `infra/*.tf`, `*.tfvars.example`, `.github/workflows/ci.yml` | — |

## Log

- 2026-11-13: step Build started.
- 2026-11-13: Build done. Lane A (workflows) and lane B (docs, Worker, origin-secret) green, tofu by the lead.
  Checks: fmt, leptosfmt, clippy ssr (`--offline`: `--locked` fails on the ../ui-lib path patch), test ssr (one
  timing flake of `smoke_runs_the_binary_as_prod_does` under load, green alone), the script tests, `tofu fmt`,
  `tofu validate` in a scratch copy. Not done here, by order: push, plan, apply.
- Open: `app/` still carries the password gate (VIZ_GATE=password, DEV_PASSWORD) and `-dev` suffix handling, and
  `deploy.yml` keeps the Worker-500 arm naming `origin-secret push` (gone): both go with prod's removal from
  Cloud Run (they are image-hash inputs, left untouched on purpose).

## Retro

Light path, one line: the brief listed the Worker files but not the scripts and CI steps naming it (no_js_test.py, ci.yml, check_workflows.py); a removal brief should grep for every reference first.
