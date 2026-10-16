# syntax=docker/dockerfile:1.7
#
# vizualisation: the server and the site (WASM, CSS, assets) in one image.
#
# From search-front/Dockerfile. What differs: one sibling (ui-lib; this
# service has no API client), the workspace's four members (`app`, `engine`,
# `layout`, `formats`) copied, the binary and output name `vizualisation`,
# and no token check: search-front ran `scripts/check-wasm-no-token.sh` and
# shipped `token-pattern` beside `smoke.sh` because it holds an API token
# that must never reach the browser; this service holds none (its secrets are
# the origin secret and, on dev, the password, both server-side env read at
# boot), so both files are dropped. The smoke test checks instead that no
# response and no log holds a secret, and that the boot refuses an unset
# VIZ_GATE.
#
# Build context: the FOLDER HOLDING `vizualisation/` AND `ui-lib/` (CI checks
# the two out side by side, `.github/actions/siblings`). The workspace reaches
# the sibling: its `[patch]` to `../ui-lib`, `app/style/main.css` (`@import`
# of `../../../ui-lib/style/theme.css`, `@source` of its sources) and
# `app/build.rs` (`#[path]` to `../../ui-lib/build/placeholders.rs`). So,
# from that folder:
#
#   docker build -f vizualisation/Dockerfile .
#
# NO CREDENTIAL IS NEEDED TO BUILD (search-front 007, ruling R1). ui-lib is a
# private git dependency, but the `[patch]` replaces it with the sibling
# checkout COPYed below and `Cargo.lock` records it as a path package: it
# holds no git source, so Cargo fetches nothing from GitHub. The GitHub App
# token lives in the CI checkout steps only; it is never an argument, a
# secret mount or a layer here. A RUN below fails the build if the lock ever
# gains a git source (and `app/tests/image_context.rs` pins the same
# property), instead of letting the build ask for a credential.
#
# There is no .dockerignore (it would live in the parent folder, outside this
# repository); every COPY below names what it takes instead.

# The image tag (`scripts/image-hash.sh`) hashes this file and `scripts/`
# with the sources: a fix here changes the tag.

# ---------------------------------------------------------------- tools
# The build tools, in their own layers so a source change does not reinstall
# them. 1.98 as search-front's image (the MSRV of the workspace is 1.88,
# checked in CI).
FROM rust:1.98-bookworm AS tools
RUN rustup target add wasm32-unknown-unknown
# cargo-leptos builds the server and the WASM and runs Tailwind. It downloads
# Tailwind's standalone binary and binaryen (wasm-opt) itself, at versions it
# pins, on first use.
ARG CARGO_LEPTOS=0.3.11
RUN cargo install cargo-leptos --version "$CARGO_LEPTOS" --locked
# wasm-bindgen-cli must match the `wasm-bindgen` crate of Cargo.lock to the
# patch version, or the glue fails at build time: it is read from the lock,
# never typed twice.
COPY vizualisation/Cargo.lock /lock/Cargo.lock
RUN WB=$(awk '/^name = "wasm-bindgen"$/ {getline; gsub(/[^0-9.]/, ""); print; exit}' /lock/Cargo.lock) \
    && test -n "$WB" \
    && echo "wasm-bindgen-cli $WB (from Cargo.lock)" \
    && cargo install wasm-bindgen-cli --version "$WB" --locked

# ---------------------------------------------------------------- builder
FROM tools AS builder
WORKDIR /src
# ui-lib: what the workspace reads (manifest, build script, locales, theme,
# sources; README.md because a source `include_str!`s it). No .git.
COPY ui-lib/Cargo.toml ui-lib/build.rs ui-lib/README.md ./ui-lib/
COPY ui-lib/build ./ui-lib/build
COPY ui-lib/locales ./ui-lib/locales
COPY ui-lib/src ./ui-lib/src
COPY ui-lib/style ./ui-lib/style
# vizualisation: the manifest, the lock and the four members (Cargo loads
# the whole workspace, so the engine and its crates are here even though the
# server does not link them yet), and the scripts (the smoke test).
COPY vizualisation/Cargo.toml vizualisation/Cargo.lock ./vizualisation/
COPY vizualisation/app ./vizualisation/app
COPY vizualisation/engine ./vizualisation/engine
COPY vizualisation/layout ./vizualisation/layout
COPY vizualisation/formats ./vizualisation/formats
COPY vizualisation/scripts ./vizualisation/scripts
WORKDIR /src/vizualisation
# The footer badge shows the branch and the short commit this image was built
# from, read at COMPILE time by `app/build.rs` (the image has no .git, so git
# says "unknown"). CI passes both with `--build-arg`. Declared here, after the
# COPYs, so a new commit does not invalidate the layers above.
ARG BUILD_BRANCH
ARG BUILD_SHA
ENV BUILD_BRANCH=$BUILD_BRANCH \
    BUILD_SHA=$BUILD_SHA
# The build needs no credential only while the lock holds no git source.
RUN ! grep -q '^source = "git+' Cargo.lock \
    || { echo "Cargo.lock holds a git source: the build would need a GitHub credential (search-front 007)"; exit 1; }
RUN cargo leptos build --release --cargo-locked \
    && mkdir /out \
    && cp target/release/vizualisation target/release/hash.txt /out/ \
    && cp -r target/site /out/site \
    && cp scripts/smoke.sh /out/

# ---------------------------------------------------------------- runtime
FROM debian:bookworm-slim
RUN useradd --system --uid 10001 --home /nonexistent --shell /usr/sbin/nologin app
WORKDIR /app
# The server reads hash.txt NEXT TO ITS OWN EXECUTABLE to name the hashed
# files (`vizualisation.<hash>.js/.wasm/.css`: the workspace sets
# `hash-files`), when LEPTOS_HASH_FILES=true (below); site/ is
# LEPTOS_SITE_ROOT.
COPY --from=builder --chown=app:app /out/vizualisation /out/hash.txt /out/smoke.sh ./
COPY --from=builder --chown=app:app /out/site ./site
# The data files, read once at boot (app/src/data.rs, VIZ_DATA_DIR): the
# image carries the very files the commit holds (design v4: baked in, loaded
# at boot). Named one by one, as every COPY here.
COPY --chown=app:app \
    vizualisation/public/graph.json \
    vizualisation/public/graph-sim.json \
    vizualisation/public/graph-sim.bin \
    vizualisation/public/graph-sim-pca.json \
    vizualisation/public/graph-sim-umap.json \
    vizualisation/public/graph-sim-tsne.json \
    vizualisation/public/translations.json \
    vizualisation/public/blurbs-en.json \
    vizualisation/public/blurbs-fr.json \
    ./data/
# `get_configuration(None)` reads these and nothing else: no Cargo.toml in
# the image. Cloud Run sends its traffic to 8080. The runtime settings that
# differ per deployment (VIZ_GATE, and the secrets ORIGIN_SECRET and, on dev,
# DEV_PASSWORD from Secret Manager) are the service's env (infra/gcp.tf),
# never the image's: the boot refuses an unset VIZ_GATE. LEPTOS_ENV=PROD
# makes it refuse to start without ORIGIN_SECRET.
ENV LEPTOS_OUTPUT_NAME=vizualisation \
    LEPTOS_SITE_ROOT=site \
    LEPTOS_SITE_PKG_DIR=pkg \
    LEPTOS_SITE_ADDR=0.0.0.0:8080 \
    LEPTOS_HASH_FILES=true \
    LEPTOS_ENV=PROD \
    VIZ_DATA_DIR=data \
    RUST_LOG=info \
    PORT=8080
USER app
EXPOSE 8080
# Smoke test, the last step: `scripts/smoke.sh` starts the server as the
# image will run it, on a free local port, with VIZ_GATE=password and a
# throwaway origin secret and dev password it makes itself (valid for this run
# only, never the real ones), and asks it what the rollout will ask:
# /health is 200 with the origin header and 403 without and says
# `ok <commit>` and each graph file's stamp, / is 401 without the password
# and 200 with it, the .wasm it links is served; then it starts it as prod
# runs it (VIZ_GATE=off, LEPTOS_ENV=PROD): / is 200 without the password and
# nothing asks for one; then it boots it without VIZ_GATE, which must exit
# 2; no response and no log of the three runs holds a secret. A
# broken build or a missing site root fails here, not on Cloud Run. Plain
# bash (/dev/tcp): the image has no curl and gains none.
RUN ./smoke.sh ./vizualisation
CMD ["./vizualisation"]
