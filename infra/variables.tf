# Only what the google provider needs for the one destroy apply (README). A
# stale local prod.tfvars (git-ignored) that still sets `envs` or
# `bootstrap_image` only gets a warning: those variables are gone.

variable "gcp_project" {
  description = "The GCP project, shared with the site, the API, learn-pali and search-front."
  type        = string
  default     = "palicanon-app"
}

variable "region" {
  type    = string
  default = "europe-west1"
}
