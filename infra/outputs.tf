# From search-front/infra/outputs.tf. What differs: no API token in
# `secrets`. `urls` is the Cloud Run URLs, as search-front's.

output "urls" {
  description = "The Cloud Run URL of each environment. It answers 403 to any request without the origin secret."
  value       = { for k, s in google_cloud_run_v2_service.app : k => s.uri }
}

output "hosts" {
  description = "The public address of each environment."
  value       = { for k, v in var.envs : k => "https://${v.host}" }
}

output "runtime_service_accounts" {
  description = "The identity each service runs as (secretAccessor on its own secrets, nothing else)."
  value       = { for k, a in google_service_account.app : k => a.email }
}

output "secrets" {
  description = "The Secret Manager secrets to fill (no version is created by tofu)."
  value = {
    for k in keys(var.envs) : k => {
      origin_secret = google_secret_manager_secret.origin[k].secret_id
    }
  }
}
