# infra

Nothing is managed here any more. Prod (map.palicanon.app) runs on the lsh-tech
server (`Lsh0x/lsh-tech`, `stacks/palicanon/prod`), and its images are on GHCR
(`ghcr.io/palicanon/vizualisation`). The Cloud Run service, its service account,
its IAM bindings, the origin secret and the Cloudflare provider are removed from
the code.

## Why the directory is still here

The tofu state (`gs://palicanon-app-tofu-state`, prefix `vizualisation`) still
holds the old Cloud Run resources. To destroy them, tofu needs the `gcs` backend
and the `google` provider, which are all `versions.tf` and `variables.tf` keep.
A stale, git-ignored `prod.tfvars` that sets `envs` or `bootstrap_image` only
gets a warning: those variables are gone.

## Order

1. Delete the Cloud Run service. Deletion protection is on in the state, so do
   it with gcloud:

   ```sh
   gcloud run services delete vizualisation-prod --region europe-west1 --project palicanon-app
   ```

2. Destroy the rest:

   ```sh
   gcloud auth application-default login
   cd infra
   tofu init
   tofu apply   # read the plan first
   ```

   The plan destroys exactly these seven addresses (the service, already
   deleted by step 1, is dropped from the state):

   - `google_cloud_run_v2_service.app["prod"]`
   - `google_cloud_run_v2_service_iam_member.public["prod"]`
   - `google_service_account.app["prod"]`
   - `google_service_account_iam_member.deployer_acts_as["prod"]`
   - `google_secret_manager_secret.origin["prod"]`
   - `google_secret_manager_secret_iam_member.app_reads_origin["prod"]`
   - `google_secret_manager_secret_iam_member.deployer_reads_origin["prod"]`

3. Afterwards, delete this directory's provider and backend, or the directory
   itself.

## What stays in GCP

Nothing of this repository's, except the state bucket `palicanon-app-tofu-state`
(prefix `vizualisation`) and the shared `github-deployer` account, which the
site's infra owns.
