# 007 (D) The data tools in Rust, stamped, checked in CI

status: open · rank: 7 · level: standard · type: refactor · request: user, 2026-10-12 ("remplace react par lepsos"), validated 2026-10-14 ("on fait la reecriture ! jaimerai bien que ce soit dans les guidelines des autres repo") · session: vizualisation-002-leptos (design dce546795076c4c v4, discussion d49d768dcc92341, process d4b32bf84ae3f40) · process: ../../docs/workflows/development-process.mmd · after: [004]

## Summary
Rank: difficulty 6 (three Node scripts ported, the sim positions through `layout`), impact 7 (every data file; a wrong build ships wrong positions) → 7.
Design: `002 design` v4 in the viewer (session vizualisation-002-leptos), validated by the user 2026-10-14. Decided by the user: Leptos SSR + hydrate on Cloud Run; ui-lib's AppShell around the map; the force layout ported to Rust; dev protected by a password inside the server; texts in Pāli on a first visit, then the interface language. Continuity (v4): the workspace, lints, ui-lib pin, i18n, rail, badge, CI and infra follow learn-pali and search-front (their files are the reference, quoted before diverging).
This task: `tools/build-data` (Rust) replaces `scripts/build-graph.mjs`, `build-similarity.mjs`, `build-translations.mjs` and `lib/canon.mjs`, writing through `formats` and laying out through `layout`, every file stamped (SuttaCentral commits, vectors sha); `tools/reduce-3d.py` stays Python (offline, not in CI) and stamps its output; the data check (Rust test in CI): every file parses, the sim family shares one vectors sha and one node set, no uid equals a route segment. The output is compared with today's files (same nodes, same pairs) before replacing them.

## Lanes

Plan by the lead, 2026-10-22, on 1ea8c11. Probes (scratch `007/lead/`, all exit 0):
- Inputs: `../vizualisation/data` (the worktree has none) — sc-data `86ab5a9` (2026-10-02), bilara-data `2d7f150` (2026-10-02); `fetch-data.sh` pins no commit (depth-1 pull of HEAD). Vectors `../pali-embedder/out/pali-lsa-kept-512.bin` sha256 `06e58647…`, `.json` `188ab4a4…`.
- **Reference = the committed files**: node v26.8.1 + ngraph.forcelayout 3.3.1 (`../vizualisation/node_modules`) re-ran the three scripts (3.4 s, 2.3 s, 17.6 s CPU): all six outputs `cmp`-identical to `public/`. Node's directory walk is byte-sorted (libuv scandir): a sorted walk reproduced graph.json, translations, blurbs identically; Rust's `read_dir` is unsorted, so the tool sorts.
- `formats` rewrites `graph.json` byte-identical; `graph-sim.json` same length, differs only by key order (`sim` after `edges` in the struct, after `types` in the JS).
- Layout: the Rust `layout` started from ngraph's own start positions (dumped from Node), 600 steps: graph.json median displacement 4.4 (p99 59, worst 196; gyration 2064), 35/3305 rounded positions identical; graph-sim.json median 12.8 (p99 106, worst 319; gyration 1326), 2/7286. Positions can never be byte-identical (jitter generator, float order); from `scatter` the arrangement differs wholly (Q1).

Comparison method (`C`): build into a temp dir, then — `graph-sim.bin`, `translations.json`, `blurbs-en.json`, `blurbs-fr.json` **byte-identical** (asserted); `graph.json`, `graph-sim.json` parsed through `formats`: every key and every node field but `x,y,z` equal, `stamp` new, `sim.sha256` new; positions reported (median, p99, worst displacement, k=10 nearest-neighbour overlap) — the list goes to the user before anything replaces `public/`.

Slices (tool crate `tools/build-data`, binary `build-data`; each test first, red, then green):

| Slice | Behaviour | Files | Tests first | After |
| --- | --- | --- | --- | --- |
| S0 scaffold | workspace member, CLI (`graph`, `sim`, `translations`, `all`; `--data`, `--vectors`, `--out`, the scripts' knobs MIN_SIM/DEFAULT_SIM/MAX_SIM/STEPS/DIM as flags), sorted walk, a clone's HEAD commit as `SourceCommit` | `Cargo.toml` (members), `Cargo.lock`, `tools/build-data/{Cargo.toml,src/main.rs,src/lib.rs,src/walk.rs,src/source.rs}` (lib.rs declares every module; each later module is its slice's file), `Dockerfile` (COPY tools), `scripts/image-hash.sh` + `app/tests/image_hash_script.rs` (input list), `.github/workflows/ci.yml` (MAP_CRATES), README Checks | `walk_is_sorted_by_bytes_as_node`, `a_clone_stamps_its_head_commit` (temp git repo), `a_missing_clone_is_an_error_naming_it`, image-hash test: a change under tools/ changes the hash | — |
| S1 canon | canon.mjs + build-graph's hierarchy: parent map, roots, names (numeric prefix stripped), acronyms, the four-form ancestor chain, lang/pitaka/collection, the 7 groups | `tools/build-data/src/canon.rs`, `tests/fixtures/sc-mini/` | `ancestors_try_the_shorter_forms`, `acronym_from_the_collection_when_the_text_has_none`, `group_of_a_sutta_is_its_nikaya_else_kn`, `names_strip_the_numeric_prefix` | S0 |
| S2 graph.json | parallels (norm, uid regex, strongest type kept, section full/resembling → sections, pli only), ids by first appearance, layout via `layout` (centring 1), component pull to the ring, JS `Math.round`, stamp `[sc-data]` | `src/parallels.rs`, `src/components.rs`, `src/graph_view.rs`, `src/jsnum.rs` | `the_strongest_type_is_kept`, `a_section_full_or_resembling_counts_as_sections`, `a_non_uid_target_is_dropped`, `ids_follow_first_appearance`, `isolated_components_ring_the_largest_keeping_their_shape`, `js_round_is_half_up` (-2.5 → -2, -0.4 → 0), `graph_json_is_stamped_with_sc_data` | S1, Q1 |
| S3 pairs | vectors (chunk mean in f64, L2, f32, empty skipped, size checked), Park-Miller sample for `lo`, thresholds from links per text (25/6/0.5, ceil/round to 0.01), pairs ≥ lo, stable sort highest first, quantised through `formats` | `src/vectors.rs`, `src/sim.rs` | `chunk_means_are_normalised_as_node`, `a_dump_of_the_wrong_size_errors`, `an_empty_text_is_skipped`, `park_miller_matches_node` (values pasted from node), `thresholds_from_links_per_text`, `given_thresholds_override`, `ties_keep_index_order` | S0 |
| S4 graph-sim.json | springs on the default threshold's links (17.5 + (1−t)·30), layout with centring 0.99, nodes `[uid, acr, en, root, g, g, 0, x,y,z]`, `sim` with `sha256` of the bin, stamp `[sc-data]` + `vectors_sha` | `src/sim_view.rs` | `springs_are_the_default_threshold_links`, `sim_graph_passes_check_family`, pair rule: `same_vectors_sha_and_same_node_set` (each its own test) | S1, S3, Q1 |
| S5 texts | translations (bilara title = header holding the Pāli name, else last header; bilara wins over legacy html; h1 cleaned; translator from _author.json, phantuananh fix; first-credit order), blurbs (en root, fr translation; a file only when non-empty), `stamp.json` (sources sc-data + bilara-data, files' sha256) | `src/translations.rs`, `src/blurbs.rs`, `src/stamp_file.rs`, `tests/fixtures/bilara-mini/` | `title_is_the_header_holding_the_pali_name`, `else_the_last_header`, `bilara_wins_over_html`, `html_title_drops_number_and_colon`, `phantuananh_is_phan_tuan_anh`, `translators_in_first_credit_order`, `no_blurb_file_for_a_language_without_blurbs`, `stamp_json_lists_every_written_file` | S0 |
| S6 compare | `C` above as an ignored test (needs `data/` + vectors, not in CI), its report posted for the user | `tools/build-data/tests/same_as_committed.rs` | the byte-identical asserts (red on a mutated port) | S2, S4, S5 |
| S7 replace (after the user's ruling on S6's list) | regenerate `public/` (6 files + `stamp.json`), the data check strict: `check_family` and `check_sha256` required, `stamp.json` `check_files`, app boot's shared-vectors check, health "no stamps before 007" flipped; no uid equals a route segment (app route table) | `public/*`, `formats/src/pairs.rs`, `formats/tests/{stamp_file,graph_files}.rs`, `app/src/data.rs`, `app/src/route.rs` (test), README "Data", `package.json` scripts, the four `.mjs` (Q3) | the transitional tests replaced red-first by the strict ones; `no_uid_is_a_route_segment` (red with a fixture uid `sim`) | S6 + user |

JS left for 008 (proposed, Q3): React `src/`, Vite/Tailwind/TS config, `package.json` + lock, `node_modules` (ngraph devDeps), `layout/tests/fixtures/gen.cjs` (drop the generator, keep `small.json`, as its header says), `infra/worker/*.js` stays. `scripts/fetch-data.sh` (sh) stays, called directly.

Overlaps: open branches — only `fix/vizualisation-proxy-target` (013, merged locally): `infra/worker/proxy{,.test}.js`, `.task/013` — none with 007. 008 (open, no branch) deletes what S7/Q3 leaves.

ASSUMPTIONS (A1 checked by probe; A2–A5 to check in their slice):
- A1 Node's walk = byte-sorted names (probe: sorted walk reproduced the files).
- A2 `vectors_sha` = sha256 of the `.bin` vectors file (Q2).
- A3 the stamp's commit = `git rev-parse HEAD` of each clone (`data/sc-data`, `data/bilara-data`), `--data` defaulting to `data/`.
- A4 the tool is a workspace member (so CI's clippy/test/rustdoc/deny cover it), hence Dockerfile COPY + image-hash input.
- A5 `regex` added for the scripts' regexes (cargo-deny: MIT/Apache).

Questions (@orchestrator):
- Q1 Positions: port ngraph's start placement (`getBestNewPosition` + ngraph.random seed 42) into `layout` so the map keeps today's arrangement (median gap 4.4 / 12.8 measured), or start from `scatter` (a new arrangement, same physics)?
- Q2 `vectors_sha`: sha256 of the `.bin` only, or of `.bin` + docs `.json`?
- Q3 Do the four `.mjs` and their `package.json` scripts go in 007 (S7) or in 008? And `gen.cjs`: drop in 008, keep `small.json`?
- Q4 `tools/reduce-3d.py` (design REDUCE): 011 (2c051ee) deleted `scripts/reduce-3d.py` and the pca/umap/tsne files; drop REDUCE from 007? The sim family is then `graph-sim.json` + `.bin`.
- Q5 `build-graph:all` (every root language, not shipped): ported or dropped? Only the Pāli graph is committed.

## Log
| Step | Result | Output |
| --- | --- | --- |
| Define | done | design v4 validated by the user 2026-10-14 |
| Plan | done | `## Lanes`: 8 slices S0–S7, comparison method C; probes: the Node scripts reproduce public/ byte-identically, positions cannot be; Q1–Q5 open |
