# 026 The footer pages move to palicanon.app

status: in review
rank: 1
level: simple
request: the coordinator, 2026-11-15: the legal texts live only on palicanon.app (www, GDPR-updated); map stops serving its own, as search-front did (its task 037)

## Summary

`/acknowledgements`, `/terms`, `/privacy`, `/legal` and `/support` (with or without a trailing slash, GET and HEAD) answer a 301 to the same page on palicanon.app in the visitor's language (ui-lib's `resolve_language`: cookie `pc_lang`, then `Accept-Language`, then English): `https://palicanon.app/<page>/`, `/fr/<page>/`, `/vi/<page>/`; `Cache-Control: no-cache`, `Vary: Cookie, Accept-Language`, no query string carried (`app/src/moved.rs`). The map has no language-prefixed paths, so there are no `/fr/…` variants to redirect. The footer links to palicanon.app directly, in the current language. The pages' code and texts (`app/src/pages/`), the Acknowledgements' translators table (`map/ui/translators.rs`, `FirstRender::credits`), the Support screenshots (`app/public/support/`) and the catalogue keys only they read are removed. `/sitemap.xml` lists the home and the texts only. `/sitemap.xml` and `/robots.txt` carry `X-Robots-Tag: noindex`. The rollout check's hydrate step loads `/no-such-page` (the 404) instead of `/privacy`.

## Log

| Step | Result |
| --- | --- |
| Red | `cargo test -p vizualisation --features ssr --lib -- router:: site_footer::`: 6 FAILED (the 301s, the cookie and query case, the noindex header, the sitemap's count, the two footer tests); `--test site_scripts rollout_check_hydrate`: 3 FAILED |
| Green | same commands: 0 failed |
| Checks | `cargo fmt --all --check`, `leptosfmt --check app`, clippy `-D warnings` (ssr all targets, hydrate wasm32, csr wasm32 lib + test, csr host all targets), `cargo test` ssr (lib 265, the integration tests; `site_scripts` 39 with `--test-threads=1`, its smoke tests time out under a parallel run's load), csr (163), rustdoc ssr, cargo-deny, the scripts' Python and shell tests: exit 0 |
| Not run locally | `cargo leptos build --release`, the fragment in headless Chrome, MSRV, coverage, tofu (CI) |
