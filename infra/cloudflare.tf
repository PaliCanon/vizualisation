# From search-front/infra/cloudflare.tf. What differs: the names
# (`vizualisation-<env>`) and the zone read through `var.zone` (this
# directory's variable of before task 005).
#
# The public name of the site: <host> → a Worker that relays to Cloud Run.
# Cloudflare sits in front (WAF, rate limits). The zone itself is the site's
# (palisearch infra).

data "cloudflare_zone" "site" {
  name = var.zone
}

resource "cloudflare_workers_script" "proxy" {
  for_each   = var.envs
  account_id = var.cloudflare_account_id
  name       = "vizualisation-${each.key}"
  content    = file("${path.module}/worker/proxy.js")
  module     = true
  # Pins the Workers runtime behaviour the script was written against.
  compatibility_date = "2026-10-08"

  plain_text_binding {
    name = "ORIGIN"
    text = google_cloud_run_v2_service.app[each.key].uri
  }

  # The public origin of the host this Worker serves: the Worker rewrites a
  # `Location` on the run.app origin onto it, and never trusts the request's
  # own Host for that. Not a secret; https (the Worker answers 500 otherwise).
  plain_text_binding {
    name = "PUBLIC_ORIGIN"
    text = "https://${each.value.host}"
  }

  # NO secret binding here (search-front 007, design v4 and ruling R4): a
  # secret declared in tofu lands in its state. The Worker's `ORIGIN_AUTH` is
  # set through the Cloudflare API by `infra/scripts/origin-secret push <env>`.
  # The provider cannot keep bindings it does not declare, so an apply that
  # re-uploads the script (a change to worker/proxy.js, to the compatibility
  # date or to ORIGIN) can drop that secret: push again after such an apply.
  # Until then the Worker answers 500 (a loud failure, never a silent one).
}

# The Worker is served on each host by a Workers Custom Domain. Cloudflare
# creates the proxied DNS record itself and issues the certificate for the
# exact host, so there is no record and no route to declare.
resource "cloudflare_workers_domain" "app" {
  for_each    = var.envs
  account_id  = var.cloudflare_account_id
  zone_id     = data.cloudflare_zone.site.id
  hostname    = each.value.host
  service     = cloudflare_workers_script.proxy[each.key].name
  environment = "production"
}
