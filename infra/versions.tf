# Nothing is managed here any more. Prod runs on ssh.lsh.tech (Lsh0x/lsh-tech,
# stacks/palicanon/prod), and its images are on GHCR. Cloud Run, its service
# account, its IAM bindings, the origin secret and the Cloudflare provider are
# removed from the code; this directory stays for ONE apply: the state still
# holds the Cloud Run resources (README), and tofu needs the backend and the
# google provider below to destroy them. After that apply, delete this
# directory's backend and provider, or the directory itself.
terraform {
  required_version = ">= 1.8"

  # Its own state, next to the site's (prefix "palicanon"), the API's
  # (prefix "pali-canon-api"), learn-pali's and search-front's in the same
  # bucket.
  backend "gcs" {
    bucket = "palicanon-app-tofu-state"
    prefix = "vizualisation"
  }

  required_providers {
    google = {
      source  = "hashicorp/google"
      version = "~> 6.0"
    }
  }
}

# Credentials come from the environment: gcloud auth application-default
# login (the state bucket, the resources to destroy). Nothing is read from a
# file in this repository.
provider "google" {
  project = var.gcp_project
  region  = var.region
}
