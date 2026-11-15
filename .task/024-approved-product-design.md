# 024 — Integrate the approved PaliCanon product design

status: implemented — local validation passed
rank: 6 (coordinated across four repositories)
request: User, 2026-11-15: "Le design est validé dans l'ensemble. Adapte-le avec ce qui existe aujourd'hui."

## Scope
Editorial shell and responsive controls around the existing Rust map engine, graph sources and selection panel.

Approved reference: `/private/tmp/palicanon-product-v2`, including the requested smaller Search type scale. Implementation uses the current applications' data and state. No new mock API, graph or progress data. English, French and Vietnamese and light/dark/system preferences remain supported.

## Coordination
Worktrees: `/private/tmp/palicanon-design-integration`; shared appearance is opt-in in ui-lib. Public app manifests must pin the exact shared-library commit before review. No release, merge or deployment requested.
Studio conversation: `g4j47w8nrm1n`; implementation thread `d327c8ebac07`, review thread `c1ea13d8ca44`.

## Validation
The native unit and integration suites passed except two initial smoke-test timeouts under concurrent machine load. All ten smoke tests, including both initially timed-out cases, passed when rerun serially. SSR, hydrate and CSR Clippy and cargo-leptos build passed. The no-JavaScript guard passed all four tests. Final real-browser checks on /mn36 at 1440/390/320 px and light/dark themes showed the real selection panel, no overflow and no browser exceptions. Closing the real selection panel and searching for MN 10 also passed, changing the route and displayed text without browser errors. The preview uses a fresh cargo-leptos server binary paired with its WASM; the test-built binary is not used.

Shared library pinned to `204dd520d44a81499d150b2d3c4028f5f405436d`. Local-only implementation; no push, merge or deployment. Detailed logs and final evidence: `/private/tmp/palicanon-design-integration/VALIDATION.md`.

## Process exception
Automatic approval review rejected an external Claude lane because private code access for that external service had not been explicitly authorized. No external lane was launched. Implementation and verification run in the primary session. Independent persona review is not claimed.

## User follow-up
Follow-up, 2026-11-15: centered the existing map search field within its header at a bounded 620 px width, keeping its responsive full-width behavior. The real /pe3 page passed a 1728 px browser check with no overflow or exceptions.
