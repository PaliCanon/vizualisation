# From search-front/infra/variables.tf. What differs: no API URL (this
# service calls no API); each environment says its gate (`gate`, the
# service's VIZ_GATE: `off`); the host is map.palicanon.app (user,
# 2026-10-16; the repository, the services and the secrets keep the name
# vizualisation). The Cloudflare account and zone variables went with the
# Worker (task 022).

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

# One entry per environment: prod only (the dev environment was removed, task
# 022). `gate` is the service's VIZ_GATE, `off` (asks nothing). A host is
# informational here (the public name is the lsh-tech tunnel's). The map is
# keyed by environment so every prod resource keeps its address (`["prod"]`).
variable "envs" {
  type = map(object({
    host          = string
    max_instances = number
    gate          = string
  }))
  default = {
    prod = {
      host          = "map.palicanon.app"
      max_instances = 3
      gate          = "off"
    }
  }

  validation {
    condition     = alltrue([for k, v in var.envs : v.gate == "off"])
    error_message = "Each environment's gate is `off` (the service's VIZ_GATE): the dev password gate is no longer deployed."
  }

  # A stale local prod.tfvars (git-ignored) may still list dev.
  validation {
    condition     = !contains(keys(var.envs), "dev")
    error_message = "The dev environment was removed (task 022): drop the `dev` entry from infra/prod.tfvars, or delete that file (the default is prod)."
  }
}
