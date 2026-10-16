# REMOVED IN TASK 008. The Cloudflare Pages site of the React map, as this
# repository declared it before task 005 (then infra/cloudflare.tf,
# variables.tf and outputs.tf), moved here unchanged so the first apply of
# the Cloud Run infra changes nothing of it: the same resources, the same
# addresses. Only two things moved elsewhere or were renamed: the zone data
# source (`data.cloudflare_zone.site`, same address, now in cloudflare.tf,
# shared with the Worker's Custom Domain) and the `urls` output, now
# `pages_urls` (`urls` is the Cloud Run URLs, as search-front's). Task 005's
# deploy.yml no longer deploys to Pages (assumption A1): the site keeps its
# last build until task 008 removes this file and the Pages project.
#
# The map was a static site (vite build -> dist/), deployed by CI with `wrangler pages deploy`
# (direct upload): a tag pushed the virtual branch `release` (production, viz.palicanon.app),
# main was the preview that dev.viz.palicanon.app points at. Dev (every preview deployment) asks
# for a password: functions/_middleware.js, the Pages secret DEV_PASSWORD on the preview
# environment only, set through the Cloudflare API from the keychain (never tofu).

variable "project" {
  description = "Cloudflare Pages project name (also its <name>.pages.dev address)."
  type        = string
  default     = "palicanon-viz"
}

# The tag deploy pushes the virtual branch `release` (Pages production) -> prod host; every
# push to main updates the `main` preview -> dev host, behind Cloudflare Access.
variable "hosts" {
  description = "Hostnames: prod (the release branch) and dev (the main branch's preview)."
  type        = object({ prod = string, dev = string })
  default     = { prod = "viz.palicanon.app", dev = "dev.viz.palicanon.app" }
}

locals {
  # "viz.palicanon.app" -> "viz", "dev.viz.palicanon.app" -> "dev.viz"
  record = { for k, h in var.hosts : k => trimsuffix(h, ".${var.zone}") }
}

# ---------------------------------------------------------------- Pages

resource "cloudflare_pages_project" "viz" {
  account_id        = var.cloudflare_account_id
  name              = var.project
  production_branch = "release"
}

resource "cloudflare_pages_domain" "prod" {
  account_id   = var.cloudflare_account_id
  project_name = cloudflare_pages_project.viz.name
  domain       = var.hosts.prod
}

# A branch alias only serves a hostname Pages knows: the dev hostname is registered too.
resource "cloudflare_pages_domain" "dev" {
  account_id   = var.cloudflare_account_id
  project_name = cloudflare_pages_project.viz.name
  domain       = var.hosts.dev
}

# ---------------------------------------------------------------- DNS

resource "cloudflare_record" "prod" {
  zone_id = data.cloudflare_zone.site.id
  name    = local.record.prod
  type    = "CNAME"
  content = "${cloudflare_pages_project.viz.name}.pages.dev"
  proxied = true
}

resource "cloudflare_record" "dev" {
  zone_id = data.cloudflare_zone.site.id
  name    = local.record.dev
  type    = "CNAME"
  content = "main.${cloudflare_pages_project.viz.name}.pages.dev"
  proxied = true
}

# dev.viz is two levels deep: outside the zone's universal certificate, but a Pages custom domain
# gets its own certificate from Pages (docs/conventions/deploy.md), so no certificate pack.

output "pages_urls" {
  value = { for k, h in var.hosts : k => "https://${h}" }
}

output "pages_project" {
  description = "For the deploy workflow (wrangler pages deploy --project-name)."
  value       = cloudflare_pages_project.viz.name
}
