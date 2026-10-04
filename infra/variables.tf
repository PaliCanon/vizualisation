variable "cloudflare_account_id" {
  description = "Cloudflare account that owns the palicanon.app zone and the Pages project."
  type        = string
}

variable "zone" {
  description = "The apex domain, held in Cloudflare DNS."
  type        = string
  default     = "palicanon.app"
}

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

variable "admin_emails" {
  description = "Who may open the dev site and the previews (Cloudflare Access allow-list)."
  type        = list(string)
}
