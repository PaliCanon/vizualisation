# From search-front/infra/variables.tf. What differs: no API URL (this
# service calls no API); each environment says its password gate (`gate`,
# the service's VIZ_GATE: `password` on dev, `off` on prod); the hosts are
# map.dev.palicanon.app and map.palicanon.app (user, 2026-10-16; the
# repository, the services and the secrets keep the name vizualisation); `cloudflare_account_id` keeps
# this repository's declaration of before task 005 (no default: the
# git-ignored terraform.tfvars gives it), and `zone` is kept from it too. The
# Pages variables (`project`, `hosts`) moved to pages.tf with what uses them.

variable "gcp_project" {
  description = "The GCP project, shared with the site, the API, learn-pali and search-front."
  type        = string
  default     = "palicanon-app"
}

variable "region" {
  type    = string
  default = "europe-west1"
}

variable "bootstrap_image" {
  description = "Placeholder image for the first apply, before CI has pushed anything."
  type        = string
  default     = "us-docker.pkg.dev/cloudrun/container/hello"
}

variable "cloudflare_account_id" {
  description = "Cloudflare account that owns the palicanon.app zone, the Workers and the Pages project."
  type        = string
}

variable "zone" {
  description = "The apex domain, held in Cloudflare DNS."
  type        = string
  default     = "palicanon.app"
}

# One entry per environment. The dev host is map.dev.palicanon.app: two
# levels deep, served by a Workers Custom Domain which issues the exact
# host's certificate (cloudflare.tf), as search-front's and learn-pali's.
# `gate` is the service's VIZ_GATE: `password` asks every visitor for the dev
# password (the secret `vizualisation-dev-password-<env>`, created for that
# environment only), `off` asks nothing. Changing a host is an edit here and
# a tofu apply.
#
# The default holds `dev` ONLY: the first apply must not stand up a public
# prod host before dev has deployed and been checked from CI. Prod is added
# afterwards, with `-var-file` or by uncommenting the block below (README,
# "Order"), then a second apply, with its own origin secret. Not before task
# 008: an order set when prod's host was viz.palicanon.app, the Pages site's
# custom domain and DNS record until then (pages.tf), which a Workers Custom
# Domain cannot take. A Workers Custom Domain cannot take any host that
# already has a record: check map.palicanon.app has none before the apply.
variable "envs" {
  type = map(object({
    host          = string
    max_instances = number
    gate          = string
  }))
  default = {
    dev = {
      host          = "map.dev.palicanon.app"
      max_instances = 2
      gate          = "password"
    }
    # prod = {
    #   host          = "map.palicanon.app"
    #   max_instances = 3
    #   gate          = "off"
    # }
  }

  validation {
    condition     = alltrue([for k, v in var.envs : contains(["password", "off"], v.gate)])
    error_message = "Each environment's gate is `password` or `off` (the service's VIZ_GATE)."
  }

  # Prod never asks for the dev password, and its runtime account never
  # reads it (no secret is created for an environment whose gate is off).
  validation {
    condition     = !contains(keys(var.envs), "prod") || try(var.envs["prod"].gate, "") == "off"
    error_message = "Prod's gate is `off`: the dev password is dev's alone."
  }
}
