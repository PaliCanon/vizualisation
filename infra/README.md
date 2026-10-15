# infra

vizualisation's own resources, in its own tofu state
(`gs://palicanon-app-tofu-state`, prefix `vizualisation`, migrated from
`suttasmap`: see "Order", step 3), inside the GCP project it shares with the
site, the API, learn-pali and search-front (`palicanon-app`,
`europe-west1`). From search-front/infra/README.md; what differs is the
password gate of dev, no API token, and the Pages site of before task 005
kept in `pages.tf` until task 008.

Per environment (the map `envs` in `variables.tf`: `dev` first, `prod` added
later, see "Order"):

| What | File | Notes |
| --- | --- | --- |
| Runtime service account `vizualisation-<env>` | `gcp.tf` | **One role per secret it reads**, on that secret only: `secretAccessor` on the origin secret and, on dev only, on the dev password. Nothing else. |
| Secret `vizualisation-origin-secret-<env>` | `gcp.tf` | The value the Worker sends as `x-origin-auth` and the service requires. Tofu creates the secret and **never a version** (the value would sit in its state); `scripts/origin-secret` makes it. |
| Secret `vizualisation-dev-password-dev` | `gcp.tf` | The password the dev gate asks for (the service's `DEV_PASSWORD`), for the environments whose `gate` is `password`: dev only. Same rule: no version in tofu; the user adds it from the keychain (step 6). Prod has no such secret, so its account cannot read it. |
| The deployer's rights | `gcp.tf` | `github-deployer` is read as a data source (the site's infra owns it), never managed here. It may **act as** the runtime account, **read the origin secret** and, on dev, **read the dev password** (the rollout check sends it; see "R3"). |
| Cloud Run service `vizualisation-<env>` | `gcp.tf` | min 0, max 2 (dev) / 3 (prod), 1 CPU, 512Mi, CPU only while a request is served, public at the Cloud Run layer. `VIZ_GATE` is a plain value (`password` on dev, `off` on prod; the boot refuses to start without it); `ORIGIN_SECRET` and, on dev, `DEV_PASSWORD` come from the secrets (`latest`, read when an instance starts). Startup probe: **TCP 8080** (see "R2"). |
| A Worker relaying to Cloud Run | `cloudflare.tf`, `worker/proxy.js` | The Worker's `ORIGIN` is the Cloud Run URL and its `PUBLIC_ORIGIN` the host's own `https://` origin (both answer 500 when missing); it adds `x-origin-auth` from its secret `ORIGIN_AUTH` and answers 500 without it. The dev password (HTTP Basic) and the gate's 401 pass through it untouched. Tested with `node --test 'infra/worker/*.test.mjs'`. |
| A Workers Custom Domain per host | `cloudflare.tf` | `viz.dev.palicanon.app` (dev), `viz.palicanon.app` (prod, added after task 008). Cloudflare creates the DNS record and the certificate. |
| `scripts/origin-secret` | `scripts/` | `create`, `push`, `check` the origin secret; tested with stub `gcloud`/`curl` (`bash infra/scripts/origin-secret.test.sh`). `check` asks the host's `/healthz` (dev's `/` is behind the password). |
| The Pages site of the React map | `pages.tf` | **Removed in task 008.** Unchanged from before task 005 (the project, its two custom domains, the two CNAME records); the first apply must show no change to it. `deploy.yml` no longer deploys to it: it keeps its last build. |

CI owns the image: tofu ignores `image`, `scaling`, `client` and
`client_version` of the service (`lifecycle.ignore_changes`, the API's rule),
so a deploy never shows up as drift, and CI never touches the environment or
the secrets. The consequence of ignoring `scaling`: a changed `max_instances`
is not applied to an existing service; change it with `gcloud run services
update --max-instances` and keep `variables.tf` in step.

The Artifact Registry repository `palicanon`, the GitHub deploy identity
(workload identity federation), the Secret Manager API and the DNS zone
belong to the site's infra, the API's infra and `platform`: only used here.

## The two gates

**The origin rule.** The service answers **403 to any request without
`x-origin-auth`** equal to its `ORIGIN_SECRET` (constant-time comparison),
on every path, `/healthz` included, so the `*.run.app` URL is no way round
Cloudflare (its WAF, its per-IP limit): only the Worker holds the value. The
same value lives in two places, set by `scripts/origin-secret`: Secret
Manager (the service reads it) and the Worker's secret store (the Worker
sends it). Neither is in tofu state.

**The dev password.** Behind the origin guard, dev asks every page and file
for the password (HTTP Basic, any user name, `401` with `WWW-Authenticate`,
never cached); `/healthz` is not behind it, so the rollout and
`origin-secret check` reach it. Prod runs the same image with `VIZ_GATE=off`
and asks nothing. It replaces the Pages function `functions/_middleware.js`
and its Pages secret `DEV_PASSWORD` (both removed in task 008), with the same
password, kept in the keychain (account `palicanon`, service
`viz-dev-password`).

### First deploy: the Worker holds the secret first

The service refuses any request without the value as soon as a revision that
requires it serves, so the Worker must already hold it. In the order below:
`origin-secret create <env>` (step 5), `tofu apply` (the Worker, step 7),
`origin-secret push <env>` and `check` (step 8), and only then the first
`deploy.yml` run (step 9). A deploy before the `push` fails its public-host
check with the Worker's 500: `deploy.yml` fails naming `origin-secret push`
and leaves the traffic where it is. Run the `push`, then re-run the job.

### Rotation

The values are read when an instance starts (the service) and when
`origin-secret push` runs (the Worker); there is no overlap, so there is a
short window in which the two disagree.

- **The origin secret, dev**: add the new version of
  `vizualisation-origin-secret-dev`, run `origin-secret push dev` (the Worker
  now sends the new value), then **start a new revision** that reads it.
  `deploy.yml` skips an image that already serves, so a rotation is not a CI
  run: force the revision with `gcloud run services update vizualisation-dev
  --region europe-west1 --revision-suffix rot-<date>`. Until that revision
  serves, the service answers the Worker with 403: a short refusal window,
  accepted on dev.
- **The origin secret, prod**: not rotated this way (search-front's rule): a
  refusal window on the public site is not accepted; prod waits for a
  two-value rotation, a follow-up task.
- **The dev password**: add the new version of
  `vizualisation-dev-password-dev` (step 6's command, after changing the
  keychain entry), then start a new revision as above. The next deploy's
  check reads the new version.

## Order: dev first, prod later

`var.envs` holds **dev only** by default. The first apply therefore stands up
`vizualisation-dev` and `viz.dev.palicanon.app` and nothing public in prod.
Prod waits for task 008: `viz.palicanon.app` is the Pages site's custom
domain and CNAME record until `pages.tf` is removed.

## Only the user (and the orchestrator for step 3), in order

Nothing here has been applied yet (task 005 wrote it, did not run it).

1. The GitHub App (Contents: read) installed on `ui-lib` and
   `vizualisation`; repository secrets `APP_ID`, `APP_PRIVATE_KEY`, variables
   `WIF_PROVIDER`, `DEPLOYER_SA`, `GCP_PROJECT` on `vizualisation` (set
   2026-10-14).
2. palisearch: `PaliCanon/vizualisation` in `github_repos`
   (`palisearch/infra/variables.tf`, one line), then its apply (the
   workload identity of the deploy).
3. **Migrate the state** from the prefix `suttasmap` to `vizualisation`
   (versions.tf changed it), then check that the Pages site is untouched:

   ```sh
   gcloud auth application-default login
   export CLOUDFLARE_API_TOKEN=$(security find-generic-password -a lsh -s cloudflare-api-token -w)
   cd infra
   tofu init -migrate-state      # answer yes: copies the state to the new prefix
   tofu plan                     # Pages: no change; the rest: to add
   ```

   The Cloudflare token needs, besides the Pages scopes it had, account
   Workers Scripts (edit) and zone Workers Routes (edit), as search-front's.
   Once the plan is read, the old prefix `suttasmap/` in the bucket can be
   deleted by hand.
4. A **targeted** apply of the secrets, the runtime service account and the
   IAM members (the Cloud Run service cannot start before the secrets have a
   version):

   ```sh
   tofu apply \
     -target=google_secret_manager_secret.origin \
     -target=google_secret_manager_secret.dev_password \
     -target=google_service_account.app \
     -target=google_secret_manager_secret_iam_member.app_reads_origin \
     -target=google_secret_manager_secret_iam_member.app_reads_dev_password \
     -target=google_secret_manager_secret_iam_member.deployer_reads_origin \
     -target=google_secret_manager_secret_iam_member.deployer_reads_dev_password \
     -target=google_service_account_iam_member.deployer_acts_as
   ```

5. Generate the origin secret (stored in Secret Manager; nothing secret is
   printed):

   ```sh
   infra/scripts/origin-secret create dev
   ```

6. Store the dev password, the value never typed, shown or saved elsewhere
   (the keychain entry the Pages secret came from):

   ```sh
   printf '%s' "$(security find-generic-password -a palicanon -s viz-dev-password -w)" \
     | gcloud secrets versions add vizualisation-dev-password-dev --project palicanon-app --data-file=-
   ```

7. The rest: `tofu apply` (Cloud Run on the placeholder image, the Worker,
   the Custom Domain; the plan shows no change to `pages.tf`'s resources).
   The first apply may take a few minutes after the Custom Domain is
   created: Cloudflare issues the host's certificate then.
8. Give the Worker its secret and check it:

   ```sh
   infra/scripts/origin-secret push dev
   infra/scripts/origin-secret check dev   # prints OK
   ```

9. Push `vizualisation` main: `deploy.yml` rolls dev; look at
   https://viz.dev.palicanon.app (it asks for the password).

The services start with the **bootstrap image**
(`us-docker.pkg.dev/cloudrun/container/hello`, `var.bootstrap_image`), which
answers on 8080 and so passes the TCP startup probe; it checks neither the
origin header nor a password, so `check` says OK before the first real
deploy proves nothing about the gates (the rollout does).

## Prod

After task 008 has removed `pages.tf`, and dev has deployed from CI and been
checked: copy `prod.tfvars.example` to `prod.tfvars` (git-ignored), and
repeat steps 4, 5, 7 and 8 with `prod` (`origin-secret create prod`, `tofu
apply -var-file=prod.tfvars`, `origin-secret push prod`; no dev password:
prod's gate is `off`, which `variables.tf` enforces). A `v*` tag then deploys
the real image.

## Rulings that shape this directory (search-front 007, kept)

- **R2, the TCP probe.** The service refuses a request without the origin
  header, a Cloud Run HTTP probe header is a plain string in the service
  resource, and the origin secret would then sit in the tofu state and in the
  revision JSON. So the startup probe is TCP on 8080; `/healthz` is checked
  in the rollout (`scripts/rollout-check.sh`) on the tagged revision, before
  traffic moves, with the header read from Secret Manager.
- **R3, the deployer reads the origin secret** (and here the dev password):
  the pre-traffic check needs the header, and on dev the password, so
  `github-deployer` has `secretAccessor` on
  `vizualisation-origin-secret-<env>` and `vizualisation-dev-password-dev`,
  those secrets only. Consequence: any repository listed in `github_repos`
  of the site's infra (workload identity) can read them.
- **R4, a re-uploaded Worker can lose its secret.** The Cloudflare provider
  has no `keep_bindings`: a `tofu apply` that changes `worker/proxy.js`, the
  compatibility date or `ORIGIN` re-uploads the script, and Cloudflare
  replaces bindings on upload unless they are kept. Rule: **run
  `infra/scripts/origin-secret push <env>` after any apply that touches the
  Worker.** The Worker answers 500 when its secret is missing, so a drop is
  loud. Declaring the secret in tofu (`cloudflare_workers_secret`) would put
  the value in the state: ruled out.
- **R6, Bot Fight.** The check from a GitHub runner to
  `viz.dev.palicanon.app` after traffic can meet Cloudflare's Bot Fight Mode
  or a managed challenge (HTTP 403 with `cf-mitigated: challenge`). The
  script tells it from the service's own 403. If it happens, the decision is
  the user's: a WAF skip rule for the runner, or drop that one check.

## Not done here

- **Cloudflare Access on dev.** Not needed: the service's own password gate
  replaces it (design v4).
- The hosts (`viz.dev.palicanon.app`, `viz.palicanon.app`) are the design's
  (task 005, v4).
