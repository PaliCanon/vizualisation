//! Production JSON output and request privacy, without a telemetry backend.
#![cfg(feature = "ssr")]

use std::io::{self, Write};
use std::sync::{Arc, Mutex};

#[derive(Clone, Default)]
struct Capture(Arc<Mutex<Vec<u8>>>);

impl Write for Capture {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn startup_failure_carries_the_deployed_tag_without_config_values() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_vizualisation"))
        .env_clear()
        .env("VIZ_GATE", "https://canary-private.invalid")
        .env("VIZ_ENV", "v9.8.7")
        .env("VIZ_SHA", "abcdef0123")
        .env("VIZ_DEPLOYMENT_ENV", "test")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(output.stderr.len(), 0);
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(!text.contains("canary-private"));
    let log: serde_json::Value = serde_json::from_str(text.trim()).unwrap();
    assert_eq!(log["service.namespace"], "palicanon");
    assert_eq!(log["service.name"], "map");
    assert_eq!(log["service.version"], "v9.8.7");
    assert_eq!(log["service.commit"], "abcdef0");
    assert_eq!(log["event.name"], "service.failed");
    assert_eq!(log["stage"], "VIZ_GATE");
}

#[tokio::test(flavor = "current_thread")]
async fn http_error_logs_use_templates_and_ignore_request_data() {
    use axum::{Router, body::Body, http::Request, routing::get};
    use tower::ServiceExt;
    use tracing::instrument::WithSubscriber;
    let output = Capture::default();
    let writer = output.clone();
    let subscriber = tracing_subscriber::fmt()
        .json()
        .flatten_event(true)
        .with_writer(move || writer.clone())
        .finish();
    let app = Router::new()
        .route(
            "/word/{word}",
            get(|| async { axum::http::StatusCode::BAD_GATEWAY }),
        )
        .layer(axum::middleware::from_fn(
            vizualisation::telemetry::server::request,
        ));
    let request = Request::get("/word/canary-word?q=canary-query")
        .header("authorization", "Bearer canary-token")
        .header("cookie", "canary-account")
        .header("x-request-id", "canary-id")
        .body(Body::empty())
        .unwrap();
    assert_eq!(
        app.oneshot(request)
            .with_subscriber(subscriber)
            .await
            .unwrap()
            .status(),
        502
    );
    let bytes = output.0.lock().unwrap();
    let text = String::from_utf8_lossy(&bytes);
    assert!(!text.contains("canary"), "{text}");
    let log: serde_json::Value = serde_json::from_str(text.trim()).unwrap();
    assert_eq!(log["http.route"], "/word/{word}");
    assert_eq!(log["http.response.status_code"], 502);
    assert_eq!(log["severity"], "ERROR");
    assert!(log["span"]["request.id"].as_str().is_some());
}

#[tokio::test(flavor = "current_thread")]
async fn real_router_logs_one_normalized_page_and_keeps_probes_quiet() {
    use axum::{body::Body, http::Request};
    use tower::ServiceExt;
    use tracing::instrument::WithSubscriber;
    let output = Capture::default();
    let writer = output.clone();
    let subscriber = tracing_subscriber::fmt()
        .json()
        .flatten_event(true)
        .with_writer(move || writer.clone())
        .finish();
    let subscriber = tracing::Dispatch::new(subscriber);
    let options = leptos::prelude::LeptosOptions::builder()
        .output_name("vizualisation")
        .build();
    let config =
        vizualisation::boot::Config::from_lookup(|key| (key == "VIZ_GATE").then(|| "off".into()))
            .unwrap();
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../public");
    let data = Arc::new(vizualisation::data::Data::load(&dir).unwrap());
    let app = vizualisation::router::router(options, config, data);
    for path in [
        "/sim/canary-private-uid?q=canary-query",
        "/health",
        "/canary-scan/.env",
    ] {
        let request = Request::get(path)
            .header("cookie", "canary-account")
            .body(Body::empty())
            .unwrap();
        let _ = app
            .clone()
            .oneshot(request)
            .with_subscriber(subscriber.clone())
            .await
            .unwrap();
    }
    let bytes = output.0.lock().unwrap();
    let text = String::from_utf8_lossy(&bytes);
    assert!(!text.contains("canary"), "{text}");
    let logs: Vec<serde_json::Value> = text
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(
        logs.len(),
        1,
        "only the admitted page, exactly once: {text}"
    );
    assert_eq!(logs[0]["event.name"], "http.request.completed");
    assert!(logs[0]["http.route"].as_str().unwrap().contains("{uid}"));
}
