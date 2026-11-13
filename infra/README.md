# infra

vizualisation's own resources, in its own tofu state
(`gs://palicanon-app-tofu-state`, prefix `vizualisation`), inside the GCP
project it shares with the site, the API, learn-pali and search-front
(`palicanon-app`, `europe-west1`). From search-front/infra/README.md.

**Only prod is left here** (task 022): the
Cloud Run service `vizualisation-prod`, deployed by a `v*` tag. Public prod
traffic is served from the lsh-tech server (`Lsh0x/lsh-tech`,
`stacks/palicanon/prod`, images by digest), not from this service; the Cloud
Run prod stays until it is removed separately.

**The pipeline, plainly.** A push to `main` runs CI only (`ci.yml`) and
deploys nothing. A tag `v*` runs `deploy.yml`: it builds the image and rolls
`vizualisation-prod` on Cloud Run. No deployment uses the app's password gate
(`VIZ_GATE=password`, `DEV_PASSWORD`): the code stays in the app, nothing sets
it.

The map `envs` in `variables.tf` holds `prod` only, by default: a plain
`tofu plan` / `tofu apply` needs no `-var-file`.

| What | File | Notes |
| --- | --- | --- |
| Runtime service account `vizualisation-prod` | `gcp.tf` | **One role per secret it reads**, on that secret only: `secretAccessor` on the origin secret. Nothing else. |
| Secret `vizualisation-origin-secret-prod` | `gcp.tf` | The value the service requires as `x-origin-auth`. Tofu creates the secret and **never a version** (the value would sit in its state); `scripts/origin-secret` makes it. |
| The deployer's rights | `gcp.tf` | `github-deployer` is read as a data source (the site's infra owns it), never managed here. It may **act as** the runtime account and **read the origin secret** (the rollout check sends it; see "R3"). |
| Cloud Run service `vizualisation-prod` | `gcp.tf` | min 0, max 3, 1 CPU, 512Mi, CPU only while a request is served, public at the Cloud Run layer. `VIZ_GATE` is a plain value (`off`; the boot refuses to start without it); `ORIGIN_SECRET` comes from the secret (`latest`, read when an instance starts); **`VIZ_ENV` and `VIZ_SHA` are written by the deploy, not by tofu**. Startup probe: **TCP 8080** (see "R2"). |
| `scripts/origin-secret` | `scripts/` | `create` and `check` the origin secret (`prod` only); tested with a stub `gcloud` (`bash infra/scripts/origin-secret.test.sh`). `check` says OK when the secret has an enabled version. |

CI owns the image: tofu ignores `image`, `env`, the revision name, `scaling`, `client` and
`client_version` of the service (`lifecycle.ignore_changes`, the API's rule),
so a deploy never shows up as drift, and CI never touches the environment or
the secrets. The release (a tag, `v0.1.8`)
is written onto the revision by the deploy with `gcloud run deploy --update-env-vars`
(never `--set-env-vars`, which would wipe `RUST_LOG`, `VIZ_GATE` and the `ORIGIN_SECRET`
reference), setting `VIZ_ENV` and `VIZ_SHA`. The ignore of the container `env` must be applied
(`tofu apply`) **before the first deploy that writes the env**, or the next apply
strips them and the badge reads "dev" with no diff shown. A later change to
`RUST_LOG`, `VIZ_GATE` or `ORIGIN_SECRET` is a `gcloud run services update` or
`tofu apply -replace`, written down here. After a `tofu apply -replace` the service
loses the release env (`VIZ_ENV`, `VIZ_SHA`: the badge and `/health` read "dev") until the next tag deploy
writes it; a push to main is CI only and restores nothing. Restore it by hand with
`gcloud run deploy <service> --region europe-west1 --update-env-vars VIZ_ENV=<tag>,VIZ_SHA=`
(the tag's name and an empty sha; this workflow has no manual trigger).
The consequence of ignoring `scaling`: a changed `max_instances`
is not applied to an existing service; change it with `gcloud run services
update --max-instances` and keep `variables.tf` in step.

The Artifact Registry repository `palicanon`, the GitHub deploy identity
(workload identity federation) and the Secret Manager API belong to the
site's infra, the API's infra and `platform`: only used here.

## Removing dev

Task 022 removed the dev environment: the Cloud Run service
`vizualisation-dev`, its Worker, its Custom Domain `map.dev.palicanon.app` and
its secrets. The next `tofu apply` destroys exactly these 12 resources, and
nothing else (read the plan first):

- `google_cloud_run_v2_service.app["dev"]`
- `google_cloud_run_v2_service_iam_member.public["dev"]`
- `google_service_account.app["dev"]`
- `google_service_account_iam_member.deployer_acts_as["dev"]`
- `google_secret_manager_secret.origin["dev"]`
- `google_secret_manager_secret_iam_member.app_reads_origin["dev"]`
- `google_secret_manager_secret_iam_member.deployer_reads_origin["dev"]`
- `google_secret_manager_secret.dev_password["dev"]`
- `google_secret_manager_secret_iam_member.app_reads_dev_password["dev"]`
- `google_secret_manager_secret_iam_member.deployer_reads_dev_password["dev"]`
- `cloudflare_workers_script.proxy["dev"]`
- `cloudflare_workers_domain.app["dev"]`

Before the apply: your local, git-ignored `infra/prod.tfvars` must **drop its
`dev` entry (or be deleted: the default is prod)**, or validation refuses the
plan; the apply no longer takes `-var-file=prod.tfvars`. The two Cloudflare
destroys need `CLOUDFLARE_API_TOKEN` in the environment (the `cloudflare`
provider block stays for this one apply and is removed afterwards):

```sh
export CLOUDFLARE_API_TOKEN=$(security find-generic-password -a lsh -s cloudflare-api-token-palicanon -w)
cd infra && tofu plan    # the 12 resources above to destroy, no change to prod
```

The dev password (keychain: account `palicanon`, service `viz-dev-password`)
is no longer used by this repository; delete that entry when you wish.

## The origin rule

The service answers **403 to any request without `x-origin-auth`** equal to
its `ORIGIN_SECRET` (constant-time comparison), on every path, `/health`
included, so the `*.run.app` URL answers nothing to a stranger. The value is
in Secret Manager only (`vizualisation-origin-secret-prod`, set by
`scripts/origin-secret create prod`; not in tofu state). The Cloudflare Worker
that used to send it is gone: the only sender is CI's rollout check, which
reads the secret (R3).

### Rotation

The value is read when an instance starts, so a rotation is a new version
plus a new revision. Prod's origin secret is not rotated by a script: a
refusal window on a served site is not accepted, so it waits for a two-value
rotation, a follow-up task.

## Order (a new project, or a rebuild)

The first-time order; prod only.

1. The GitHub App (Contents: read) installed on `ui-lib` and
   `vizualisation`; repository secrets `APP_ID`, `APP_PRIVATE_KEY`, variables
   `WIF_PROVIDER`, `DEPLOYER_SA`, `GCP_PROJECT` on `vizualisation` (set
   2026-10-14).
2. palisearch: `PaliCanon/vizualisation` in `github_repos`
   (`palisearch/infra/variables.tf`, one line), then its apply (the
   workload identity of the deploy).
3. A **targeted** apply of the secret, the runtime service account and the
   IAM members (the Cloud Run service cannot start before the secret has a
   version):

   ```sh
   gcloud auth application-default login
   cd infra
   tofu init
   tofu apply \
     -target=google_secret_manager_secret.origin \
     -target=google_service_account.app \
     -target=google_secret_manager_secret_iam_member.app_reads_origin \
     -target=google_secret_manager_secret_iam_member.deployer_reads_origin \
     -target=google_service_account_iam_member.deployer_acts_as
   ```

4. Generate the origin secret (stored in Secret Manager; nothing secret is
   printed), and check it:

   ```sh
   infra/scripts/origin-secret create prod
   infra/scripts/origin-secret check prod   # prints OK
   ```

5. The rest: `tofu apply` (Cloud Run on the placeholder image).
6. Push a `v*` tag: `deploy.yml` builds the image and rolls prod.

The service starts with the **bootstrap image**
(`us-docker.pkg.dev/cloudrun/container/hello`, `var.bootstrap_image`), which
answers on 8080 and so passes the TCP startup probe; it does not check the
origin header, so `check` saying OK before the first real deploy proves
nothing about the guard (the rollout does).

## Task 008, the Pages removal

Task 008 deleted `pages.tf` (the Pages site of the React map, its custom
domains and their DNS records); it was applied. Afterwards the Pages scopes
of the Cloudflare token and the `CLOUDFLARE_*` repository secrets (the old
Pages deploy's) are no longer used.

## Rulings that shape this directory (search-front 007, kept)

- **R2, the TCP probe.** The service refuses a request without the origin
  header, a Cloud Run HTTP probe header is a plain string in the service
  resource, and the origin secret would then sit in the tofu state and in the
  revision JSON. So the startup probe is TCP on 8080; `/health` is checked
  in the rollout (`scripts/rollout-check.sh`) on the tagged revision, before
  traffic moves, with the header read from Secret Manager.
- **R3, the deployer reads the origin secret**: the pre-traffic check needs
  the header, so `github-deployer` has `secretAccessor` on
  `vizualisation-origin-secret-prod`, that secret only. Consequence: any
  repository listed in `github_repos` of the site's infra (workload identity)
  can read it.
- **R6, Bot Fight.** The check from a GitHub runner to the public host after
  traffic can meet Cloudflare's Bot Fight Mode or a managed challenge (HTTP
  403 with `cf-mitigated: challenge`). The script tells it from the service's
  own 403. If it happens, the decision is the user's: a WAF skip rule for the
  runner, or drop that one check.

(R4, "a re-uploaded Worker can lose its secret", went with the Worker.)
