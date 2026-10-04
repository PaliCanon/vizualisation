terraform {
  required_version = ">= 1.8"

  # Same bucket as palicanon.app's infra (palisearch/infra), its own prefix: one state per service.
  backend "gcs" {
    bucket = "palicanon-app-tofu-state"
    prefix = "suttasmap"
  }

  required_providers {
    cloudflare = {
      source  = "cloudflare/cloudflare"
      version = "~> 4.52"
    }
  }
}

# Credentials come from the environment, never from files in the repo:
#   gcloud auth application-default login        (the state bucket)
#   export CLOUDFLARE_API_TOKEN=…   (zone: DNS, Access; account: Pages, Access)
provider "cloudflare" {}
