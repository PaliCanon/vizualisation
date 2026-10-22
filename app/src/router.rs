//! The axum router (design v4, the server's layers): the origin guard
//! outermost, on every path; `/health` behind it alone; then the password
//! gate around the pages and the files (the site's files answered first, the
//! app for every other path). The signature takes the boot
//! [`Config`], so `main` cannot forget a layer.
//!
//! The data files are served at their hashed URLs (`/data/…`,
//! [`crate::manifest`]), behind the gate like every file.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use leptos::prelude::LeptosOptions;
use leptos_axum::{LeptosRoutes, file_and_error_handler, generate_route_list};
use tower_http::services::ServeDir;

use crate::app::{App, shell};
use crate::boot::Config;
use crate::data::{Data, Served};
use crate::manifest::content_type;
use crate::{gate, origin};

/// A data file's URL names its content: it never changes, any cache may keep
/// it for good.
const IMMUTABLE: &str = "public, max-age=31536000, immutable";

/// The health probe's path. Not `/healthz`: Cloud Run reserves the paths
/// ending in `z` and answers them itself.
pub const HEALTH_PATH: &str = "/health";

/// The health probe: behind the origin guard only, so the rollout check
/// reaches it through the Worker without the dev password. Public data only:
/// the deploy's release (first line: `ok <env>`, then ` <sha>` for a branch
/// build, [`ReleaseInfo::health_line`]) and what each data file was built
/// from.
async fn health(axum::extract::State(data): axum::extract::State<Arc<Data>>) -> String {
    let mut lines = vec![crate::release::ReleaseInfo::server().health_line()];
    lines.extend(data.stamp_lines());
    lines.join("\n") + "\n"
}

/// `sitemap.xml`, as `crate::head::sitemap` writes it from the text index.
async fn sitemap(
    State(served): State<Served>,
) -> ([(axum::http::HeaderName, &'static str); 1], String) {
    (
        [(axum::http::header::CONTENT_TYPE, "application/xml")],
        crate::head::sitemap(&served.texts),
    )
}

/// `/data/{file}`: the data file whose hashed name `file` is, as read at
/// boot; a 404 for any other name, a stale hash included.
async fn data_file(State(served): State<Served>, Path(file): Path<String>) -> Response {
    let found = served
        .manifest
        .file_at(&file)
        .and_then(|name| Some((name, served.data.raw.get(name)?)));
    let Some((name, bytes)) = found else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let mut response = bytes.clone().into_response();
    let headers = response.headers_mut();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static(content_type(name)),
    );
    // Behind the dev password the gate replaces this with `private, no-store`
    // (`gate::guard` runs after the handler): a cache must not keep what
    // the password guards, so in dev the gate's header wins, on purpose.
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static(IMMUTABLE));
    response
}

/// The service's router: the pages, the static files and the data files
/// behind the gate, the probe beside them, the origin guard around
/// everything; `Served` (the data, its manifest, the text index) in every
/// route's context (the map screen reads it, task 006).
pub fn router(options: LeptosOptions, config: Config, data: Arc<Data>) -> axum::Router {
    let served = Served::new(Arc::clone(&data));
    let routes = generate_route_list(App);
    let site_root = options.site_root.to_string();
    let app = axum::Router::new()
        .leptos_routes_with_context(
            &options,
            routes,
            {
                let served = served.clone();
                move || leptos::prelude::provide_context(served.clone())
            },
            {
                let options = options.clone();
                move || shell(options.clone())
            },
        )
        .fallback(file_and_error_handler(shell))
        .with_state(options);
    // The site's files first, the app for everything else: `/:uid` (an open
    // text) would otherwise answer `/favicon.ico` or `/lotus.png` with a page.
    let files = ServeDir::new(site_root)
        .append_index_html_on_directories(false)
        .call_fallback_on_method_not_allowed(true)
        .fallback(app);
    let site = axum::Router::new()
        .route("/sitemap.xml", get(sitemap).with_state(served.clone()))
        .route("/data/{file}", get(data_file).with_state(served))
        .fallback_service(files)
        .layer(axum::middleware::from_fn_with_state(
            Arc::new(config.gate),
            gate::guard,
        ));
    axum::Router::new()
        // `/health`, not `/healthz`: Cloud Run's front end keeps the paths ending in
        // `z` (`/healthz`, `/livez`, `/readyz`) for itself and answers them with its
        // own 404, so the container never sees them (005's first rollout, 2026-10-16).
        .route(HEALTH_PATH, get(health).with_state(data))
        .merge(site)
        .layer(axum::middleware::from_fn_with_state(
            Arc::new(config.origin),
            origin::guard,
        ))
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::http::{Request, StatusCode, header};
    use base64::Engine as _;
    use base64::engine::general_purpose::STANDARD;
    use tower::ServiceExt as _;

    use super::*;
    use crate::gate::{Gate, Password};
    use crate::origin::{ORIGIN_HEADER, Origin, OriginSecret};

    /// The committed data, read once for every test of this module.
    fn data() -> Arc<Data> {
        static DATA: std::sync::OnceLock<Arc<Data>> = std::sync::OnceLock::new();
        Arc::clone(DATA.get_or_init(|| {
            let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../public");
            Arc::new(Data::load(&dir).expect("the committed data"))
        }))
    }

    const PASSWORD: &str = "gate_canary_router_1b7c";
    const SECRET: &str = "origin_canary_router_9a4d";

    fn dev() -> axum::Router {
        router(
            LeptosOptions::builder()
                .output_name("vizualisation")
                .build(),
            Config {
                gate: Gate::Password(Password::new(PASSWORD)),
                origin: Origin::Checked(OriginSecret::new(SECRET)),
            },
            data(),
        )
    }

    async fn status(path: &str, origin: bool, password: bool) -> (StatusCode, String) {
        let mut request = Request::get(path);
        if origin {
            request = request.header(ORIGIN_HEADER, SECRET);
        }
        if password {
            let pair = STANDARD.encode(format!("dev:{PASSWORD}"));
            request = request.header(header::AUTHORIZATION, format!("Basic {pair}"));
        }
        let response = dev()
            .oneshot(request.body(Body::empty()).unwrap())
            .await
            .unwrap();
        let status = response.status();
        let headers = format!("{:?}", response.headers());
        (status, headers)
    }

    #[test]
    fn the_health_path_is_not_one_cloud_run_keeps() {
        assert!(!HEALTH_PATH.ends_with('z'), "{HEALTH_PATH}");
    }

    #[tokio::test]
    async fn the_layers_answer_in_the_designs_order() {
        // (path, with the origin header, with the password, status)
        let cases = [
            ("/health", false, false, StatusCode::FORBIDDEN),
            ("/health", false, true, StatusCode::FORBIDDEN),
            ("/health", true, false, StatusCode::OK),
            ("/", false, true, StatusCode::FORBIDDEN),
            ("/", true, false, StatusCode::UNAUTHORIZED),
            (
                "/pkg/vizualisation.wasm",
                true,
                false,
                StatusCode::UNAUTHORIZED,
            ),
            ("/", true, true, StatusCode::OK),
        ];
        for (path, origin, password, expected) in cases {
            let (status, headers) = status(path, origin, password).await;
            assert_eq!(
                status, expected,
                "{path} origin={origin} password={password}"
            );
            assert!(!headers.contains(PASSWORD) && !headers.contains(SECRET));
        }
    }

    /// A site root holding one image, as `cargo leptos build` leaves the
    /// assets (`app/public/`) at the root of `target/site`.
    fn site_with_an_image() -> (std::path::PathBuf, Vec<u8>) {
        let root = std::env::temp_dir().join(format!("viz-router-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let png = b"\x89PNG\r\n\x1a\n not really an image".to_vec();
        std::fs::write(root.join("lotus.png"), &png).unwrap();
        (root, png)
    }

    #[tokio::test]
    async fn a_file_at_the_root_is_served_not_taken_for_a_text() {
        let (root, png) = site_with_an_image();
        let app = router(
            LeptosOptions::builder()
                .output_name("vizualisation")
                .site_root(root.to_string_lossy().into_owned())
                .build(),
            Config {
                gate: Gate::Off,
                origin: Origin::Open,
            },
            data(),
        );
        let response = app
            .oneshot(Request::get("/lotus.png").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()[header::CONTENT_TYPE], "image/png");
        let body = axum::body::to_bytes(response.into_body(), 1 << 20)
            .await
            .unwrap();
        assert_eq!(body.as_ref(), png.as_slice());
    }

    #[tokio::test]
    async fn the_sitemap_is_served_as_xml_behind_the_gate() {
        let (code, headers) = status("/sitemap.xml", true, true).await;
        assert_eq!(code, StatusCode::OK);
        assert!(headers.contains("application/xml"), "{headers}");
        let (code, _) = status("/sitemap.xml", true, false).await;
        assert_eq!(code, StatusCode::UNAUTHORIZED);
    }

    /// The probe says which build answers and what each graph file was built
    /// from: `ok <sha>`, then the five stamp lines, nothing else.
    #[tokio::test]
    async fn the_probe_names_the_build_and_the_data_stamps() {
        let response = dev()
            .oneshot(
                Request::get("/health")
                    .header(ORIGIN_HEADER, SECRET)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), 1 << 16)
            .await
            .unwrap();
        let body = String::from_utf8(body.to_vec()).expect("utf-8");
        let lines: Vec<&str> = body.lines().collect();
        let stamps = data().stamp_lines();
        assert_eq!(stamps.len(), 2, "two graph files");
        assert_eq!(lines.len(), 3, "{body}");
        assert_eq!(
            lines[0],
            crate::release::ReleaseInfo::server().health_line(),
            "{body}"
        );
        assert!(lines[0].starts_with("ok "), "{body}");
        // The test process sets no `VIZ_ENV`: a local run (the env-driven
        // line is checked on the real binary, `tests/boot_process.rs`).
        assert_eq!(lines[0], "ok dev", "{body}");
        assert_eq!(&lines[1..], stamps.as_slice(), "{body}");
    }

    /// The URL a data file of the committed data should be served at,
    /// computed here from the file's bytes, not read from the manifest.
    fn hashed(name: &str) -> (String, Vec<u8>) {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../public");
        let bytes = std::fs::read(dir.join(name)).unwrap();
        let hash = &map_formats::pairs::sha256_hex(&bytes)[..16];
        let (stem, ext) = name.rsplit_once('.').unwrap();
        (format!("/data/{stem}.{hash}.{ext}"), bytes)
    }

    /// The prod router: no gate, no origin check.
    fn prod() -> axum::Router {
        router(
            LeptosOptions::builder()
                .output_name("vizualisation")
                .build(),
            Config {
                gate: Gate::Off,
                origin: Origin::Open,
            },
            data(),
        )
    }

    async fn get_from(app: axum::Router, path: &str) -> axum::response::Response {
        app.oneshot(Request::get(path).body(Body::empty()).unwrap())
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn a_data_file_is_served_at_its_hashed_url_and_cached_for_good() {
        for name in crate::data::FILES {
            let (url, bytes) = hashed(name);
            let response = get_from(prod(), &url).await;
            assert_eq!(response.status(), StatusCode::OK, "{url}");
            let kind = if name == "graph-sim.bin" {
                "application/octet-stream"
            } else {
                "application/json"
            };
            assert_eq!(response.headers()[header::CONTENT_TYPE], kind, "{url}");
            assert_eq!(
                response.headers()[header::CACHE_CONTROL],
                "public, max-age=31536000, immutable",
                "{url}"
            );
            let body = axum::body::to_bytes(response.into_body(), 1 << 24)
                .await
                .unwrap();
            assert!(
                body.as_ref() == bytes.as_slice(),
                "{url}: the bytes as committed"
            );
        }
    }

    #[tokio::test]
    async fn an_unknown_or_stale_hash_is_a_404() {
        let (url, _) = hashed("graph.json");
        let (sim, _) = hashed("graph-sim.bin");
        let hash = url
            .trim_start_matches("/data/graph.")
            .trim_end_matches(".json");
        assert_eq!(
            get_from(prod(), &url).await.status(),
            StatusCode::OK,
            "{url}"
        );
        let cases = [
            "/data/graph.json".to_owned(),
            "/data/graph.0000000000000000.json".to_owned(),
            format!("/data/graph.{}.json", &hash[..15]),
            format!("/data/graph.{}.json", hash.to_uppercase()),
            format!("/data/graph.{hash}.bin"),
            format!("/data/nope.{hash}.json"),
            format!("{url}.gz"),
            sim.replace("graph-sim.", "graph."),
            format!("/data/x/{}", url.trim_start_matches("/data/")),
        ];
        for path in cases {
            assert_eq!(
                get_from(prod(), &path).await.status(),
                StatusCode::NOT_FOUND,
                "{path}"
            );
        }
    }

    #[tokio::test]
    async fn the_data_routes_are_behind_the_gate() {
        let (url, _) = hashed("graph-sim.bin");
        let (code, _) = status(&url, false, true).await;
        assert_eq!(code, StatusCode::FORBIDDEN, "no origin header");
        let (code, _) = status(&url, true, false).await;
        assert_eq!(code, StatusCode::UNAUTHORIZED, "no password");
        let (code, headers) = status(&url, true, true).await;
        assert_eq!(code, StatusCode::OK, "{headers}");
        // Behind the password the gate's header wins over the immutable one.
        assert!(headers.contains("private, no-store"), "{headers}");
        assert!(!headers.contains("immutable"), "{headers}");
    }

    /// Every data file behind the dev password has one `Cache-Control`, the
    /// gate's `private, no-store`, never `immutable`; with the gate off (prod)
    /// its one header is `IMMUTABLE`.
    #[tokio::test]
    async fn a_data_file_is_private_behind_the_gate_and_immutable_without() {
        let pair = STANDARD.encode(format!("dev:{PASSWORD}"));
        for name in crate::data::FILES {
            let (url, _) = hashed(name);
            let request = Request::get(&url)
                .header(ORIGIN_HEADER, SECRET)
                .header(header::AUTHORIZATION, format!("Basic {pair}"))
                .body(Body::empty())
                .unwrap();
            let gated = dev().oneshot(request).await.unwrap();
            assert_eq!(gated.status(), StatusCode::OK, "{url}");
            let values: Vec<_> = gated
                .headers()
                .get_all(header::CACHE_CONTROL)
                .iter()
                .collect();
            assert_eq!(values, ["private, no-store"], "{url}");
            let open = get_from(prod(), &url).await;
            let values: Vec<_> = open
                .headers()
                .get_all(header::CACHE_CONTROL)
                .iter()
                .collect();
            assert_eq!(values, [IMMUTABLE], "{url}");
        }
    }

    #[tokio::test]
    async fn a_page_behind_the_password_is_not_cached() {
        let (_, headers) = status("/", true, true).await;
        assert!(headers.contains("private, no-store"), "{headers}");
    }

    /// A text's address whose uid no view has is a 404, behind and in
    /// front of a view, with the not-found page, not indexed; a uid one view
    /// has is a page (200), whichever view the address names. The test views
    /// removed in 011 (PCA, UMAP, t-SNE) are no view: their addresses are 404s.
    #[tokio::test]
    async fn a_uid_in_no_view_is_a_404() {
        for path in [
            "/zz999",
            "/sim/zz999",
            "/tsne/mn99999",
            "/pca",
            "/umap",
            "/tsne",
            "/pca/mn10",
            "/umap/ds1.1",
            "/tsne/mn10",
        ] {
            let response = get_from(prod(), path).await;
            assert_eq!(response.status(), StatusCode::NOT_FOUND, "{path}");
            let body = axum::body::to_bytes(response.into_body(), 1 << 24)
                .await
                .unwrap();
            let body = String::from_utf8(body.to_vec()).unwrap();
            assert!(
                body.contains(r#"<meta name="robots" content="noindex">"#),
                "{path}: {body}"
            );
        }
        for path in ["/mn10", "/sim/mn10", "/ds1.1", "/sim/ds1.1"] {
            let response = get_from(prod(), path).await;
            assert_eq!(response.status(), StatusCode::OK, "{path}");
        }
    }

    /// A trailing slash past a text or a view is a 404 with the not-found page, not indexed,
    /// no panel: what main's SSR answered before the fragment (read on main at 75f3173:
    /// `/mn10/`, `/sim/mn10/` and `/sim/` 404, `/mn10` and `/sim/mn10` 200; 012 review
    /// #10554).
    #[tokio::test]
    async fn a_trailing_slash_is_a_404_as_before_the_fragment() {
        for path in ["/sim/mn10/", "/mn10/", "/sim/"] {
            let response = get_from(prod(), path).await;
            assert_eq!(response.status(), StatusCode::NOT_FOUND, "{path}");
            let body = axum::body::to_bytes(response.into_body(), 1 << 24)
                .await
                .unwrap();
            let body = String::from_utf8(body.to_vec()).unwrap();
            assert!(
                body.contains(r#"<meta name="robots" content="noindex">"#),
                "{path}: {body}"
            );
            assert!(!body.contains("Read on SuttaCentral"), "{path}: {body}");
        }
    }

    /// The sitemap lists the texts too: one `<loc>` per text, at its
    /// canonical address, after the home and the pages.
    #[tokio::test]
    async fn the_sitemap_names_the_texts() {
        let response = get_from(prod(), "/sitemap.xml").await;
        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), 1 << 24)
            .await
            .unwrap();
        let body = String::from_utf8(body.to_vec()).unwrap();
        assert!(
            body.contains("<loc>https://map.palicanon.app/mn10</loc>"),
            "{body}"
        );
        assert_eq!(body.matches("<loc>").count(), 1 + 5 + 8268);
    }
}
