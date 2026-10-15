//! The axum router (design v4, the server's layers): the origin guard
//! outermost, on every path; `/healthz` behind it alone; then the password
//! gate around the pages and the files (the site's files answered first, the
//! app for every other path). The signature takes the boot
//! [`Config`], so `main` cannot forget a layer.

use std::sync::Arc;

use axum::routing::get;
use leptos::prelude::LeptosOptions;
use leptos_axum::{LeptosRoutes, file_and_error_handler, generate_route_list};
use tower_http::services::ServeDir;

use crate::app::{App, shell};
use crate::boot::Config;
use crate::data::Data;
use crate::{gate, origin};

/// The short commit this build came from (`build.rs`).
const BUILD_SHA: &str = env!("VIZUALISATION_GIT_SHA");

/// The health probe: behind the origin guard only, so the rollout check
/// reaches it through the Worker without the dev password. Public data only:
/// the build's commit and what each data file was built from.
async fn healthz(axum::extract::State(data): axum::extract::State<Arc<Data>>) -> String {
    let mut lines = vec![format!("ok {BUILD_SHA}")];
    lines.extend(data.stamp_lines());
    lines.join("\n") + "\n"
}

/// `sitemap.xml`, as [`crate::head::sitemap`] writes it.
async fn sitemap() -> ([(axum::http::HeaderName, &'static str); 1], String) {
    (
        [(axum::http::header::CONTENT_TYPE, "application/xml")],
        crate::head::sitemap(),
    )
}

/// The service's router: the pages and the static files behind the gate, the
/// probe beside them, the origin guard around everything; `data` in every
/// route's context (the map screen reads it, task 006).
pub fn router(options: LeptosOptions, config: Config, data: Arc<Data>) -> axum::Router {
    let routes = generate_route_list(App);
    let site_root = options.site_root.to_string();
    let app = axum::Router::new()
        .leptos_routes_with_context(
            &options,
            routes,
            {
                let data = Arc::clone(&data);
                move || leptos::prelude::provide_context(Arc::clone(&data))
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
        .route("/sitemap.xml", get(sitemap))
        .fallback_service(files)
        .layer(axum::middleware::from_fn_with_state(
            Arc::new(config.gate),
            gate::guard,
        ));
    axum::Router::new()
        .route("/healthz", get(healthz).with_state(data))
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

    #[tokio::test]
    async fn the_layers_answer_in_the_designs_order() {
        // (path, with the origin header, with the password, status)
        let cases = [
            ("/healthz", false, false, StatusCode::FORBIDDEN),
            ("/healthz", false, true, StatusCode::FORBIDDEN),
            ("/healthz", true, false, StatusCode::OK),
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

    #[tokio::test]
    async fn a_page_behind_the_password_is_not_cached() {
        let (_, headers) = status("/", true, true).await;
        assert!(headers.contains("private, no-store"), "{headers}");
    }
}
