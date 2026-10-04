# The map is a static site (vite build -> dist/), deployed by CI with `wrangler pages deploy`
# (direct upload): a tag pushes the virtual branch `release` (production, viz.palicanon.app),
# main is the preview that dev.viz.palicanon.app points at. Dev (every preview deployment) asks
# for a password: functions/_middleware.js, the Pages secret DEV_PASSWORD on the preview
# environment only, set through the Cloudflare API from the keychain (never tofu).

data "cloudflare_zone" "site" {
  name = var.zone
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
