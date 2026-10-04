output "urls" {
  value = { for k, h in var.hosts : k => "https://${h}" }
}

output "pages_project" {
  description = "For the deploy workflow (wrangler pages deploy --project-name)."
  value       = cloudflare_pages_project.viz.name
}
