# From search-front/infra/versions.tf. What differs: the state prefix
# (`vizualisation`, migrated from `suttasmap` with `tofu init -migrate-state`,
# README "Order").
terraform {
  required_version = ">= 1.8"

  # Its own state, next to the site's (prefix "palicanon"), the API's
  # (prefix "pali-canon-api"), learn-pali's and search-front's in the same
  # bucket. Was "suttasmap" until task 005.
  backend "gcs" {
    bucket = "palicanon-app-tofu-state"
    prefix = "vizualisation"
  }

  required_providers {
    google = {
      source  = "hashicorp/google"
      version = "~> 6.0"
    }
    cloudflare = {
      source  = "cloudflare/cloudflare"
      version = "~> 4.52"
    }
  }
}

# Credentials come from the environment: gcloud auth application-default
# login (the state bucket, the GCP resources), and CLOUDFLARE_API_TOKEN (zone
# DNS and Workers routes, account Workers scripts). Nothing is read from a
# file in this repository.
provider "google" {
  project = var.gcp_project
  region  = var.region
}

provider "cloudflare" {}
