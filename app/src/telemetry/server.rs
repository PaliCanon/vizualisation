//! The server's JSON sink and sanitized HTTP summaries.

use std::sync::OnceLock;
use std::time::Instant;

use tracing_subscriber::{Layer, filter::filter_fn, prelude::*};

use super::{Identity, event};
use crate::release::ReleaseInfo;

pub(super) fn identity() -> &'static Identity {
    static IDENTITY: OnceLock<Identity> = OnceLock::new();
    IDENTITY.get_or_init(|| Identity {
        environment: deployment(|key| std::env::var(key).ok()),
        version: ReleaseInfo::server().env.clone(),
        commit: ReleaseInfo::server().sha.clone(),
        runtime: "server",
    })
}

/// Deployment environment, independent of the tag/branch named by the release.
pub fn environment() -> &'static str {
    &identity().environment
}

fn deployment(get: impl Fn(&str) -> Option<String>) -> String {
    if let Some(value) = get("VIZ_DEPLOYMENT_ENV") {
        let value = value.trim();
        if !value.is_empty()
            && value.len() <= 64
            && value
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
        {
            return value.to_owned();
        }
        return "unknown".into();
    }
    match get("K_SERVICE") {
        Some(name) if name.ends_with("-prod") => "production",
        Some(name) if name.ends_with("-dev") => "development",
        Some(_) => "unknown",
        None => "development",
    }
    .into()
}

pub(crate) fn layer<W>(writer: W) -> impl Layer<tracing_subscriber::Registry>
where
    W: for<'a> tracing_subscriber::fmt::MakeWriter<'a> + Send + Sync + 'static,
{
    let level = std::env::var("VIZ_LOG_LEVEL")
        .ok()
        .and_then(|value| value.parse::<tracing::level_filters::LevelFilter>().ok())
        .unwrap_or(tracing::level_filters::LevelFilter::INFO);
    tracing_subscriber::fmt::layer()
        .json()
        .flatten_event(true)
        .with_span_list(false)
        .with_writer(writer)
        .with_filter(filter_fn(move |metadata| {
            metadata.target() == "map"
                && tracing::level_filters::LevelFilter::from_level(*metadata.level()) <= level
        }))
}

/// Installs the executable's JSON stdout subscriber and a payload-free panic hook.
pub fn init() {
    tracing_subscriber::registry()
        .with(layer(std::io::stdout))
        .init();
    std::panic::set_hook(Box::new(super::panic));
}

/// Startup success, including the tag and commit from the deploy's release.
pub fn started(origin_checked: bool, password_gate: bool, elapsed: std::time::Duration) {
    event!(
        INFO,
        "service.started",
        origin.checked = origin_checked,
        gate.password = password_gate,
        duration_ms = elapsed.as_secs_f64() * 1000.0
    );
}

/// One successful load of the committed data files, no path or content.
pub fn data_ready(files: usize, elapsed: std::time::Duration) {
    event!(
        INFO,
        "map.data.ready",
        files,
        duration_ms = elapsed.as_secs_f64() * 1000.0
    );
}

/// Name a failed asset only if its basename is in the map's closed catalogue.
pub fn data_failed(path: &std::path::Path) {
    super::data_failed(
        path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(""),
        "data_file",
        None,
    );
}

/// Startup or serving failure, with a bounded stage and no error payload.
pub fn failed(stage: &'static str) {
    event!(ERROR, "service.failed", stage);
}

/// A normal end of serving, following a termination signal.
pub fn stopped() {
    event!(INFO, "service.stopped");
}

/// Refusal reporting is bounded independently of request volume.
pub fn origin_refused() {
    static LIMIT: std::sync::Mutex<Option<Instant>> = std::sync::Mutex::new(None);
    refusal(&LIMIT, false);
}

/// Password refusals have their own bounded reporting budget.
pub fn gate_refused() {
    static LIMIT: std::sync::Mutex<Option<Instant>> = std::sync::Mutex::new(None);
    refusal(&LIMIT, true);
}

fn refusal(limit: &std::sync::Mutex<Option<Instant>>, gate: bool) {
    let mut last = limit
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let now = Instant::now();
    if last.is_none_or(|last| now.duration_since(last).as_secs() >= 60) {
        *last = Some(now);
        if gate {
            event!(WARN, "gate.refused", reason = "invalid_credentials");
        } else {
            event!(WARN, "origin.refused", reason = "invalid_origin_auth");
        }
    }
}

/// Waits for SIGTERM (Cloud Run) or Ctrl-C, allowing in-flight requests to finish.
pub fn shutdown() -> impl std::future::Future<Output = ()> {
    // Register before binding the listener, not on the future's first poll.
    #[cfg(unix)]
    let termination = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate());
    async move {
        #[cfg(unix)]
        if let Ok(mut term) = termination {
            tokio::select! {
                _ = term.recv() => {},
                _ = tokio::signal::ctrl_c() => {},
            }
            return;
        }
        let _ = tokio::signal::ctrl_c().await;
    }
}

/// Emits one summary per admitted request, with a normalized route, never a URL.
pub async fn request(
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    use tracing::Instrument;
    let route = request
        .extensions()
        .get::<axum::extract::MatchedPath>()
        .map_or("unmatched", axum::extract::MatchedPath::as_str)
        .to_owned();
    let method = method(request.method());
    let id = next_request_id();
    let span = tracing::info_span!(target: "map", "http.request", request.id = %id);
    async move {
        let start = Instant::now();
        let response = next.run(request).await;
        let status = response.status().as_u16();
        let duration = start.elapsed().as_secs_f64() * 1000.0;
        if status >= 500 {
            event!(
                ERROR,
                "http.request.completed",
                http.route = route,
                http.request.method = method,
                http.response.status_code = status,
                duration_ms = duration
            );
        } else if method == "HEAD" || route == "unmatched" || route == "/health" {
            event!(
                DEBUG,
                "http.request.completed",
                http.route = route,
                http.request.method = method,
                http.response.status_code = status,
                duration_ms = duration
            );
        } else {
            event!(
                INFO,
                "http.request.completed",
                http.route = route,
                http.request.method = method,
                http.response.status_code = status,
                duration_ms = duration
            );
        }
        response
    }
    .instrument(span)
    .await
}

fn method(method: &axum::http::Method) -> &'static str {
    match method.as_str() {
        "GET" => "GET",
        "POST" => "POST",
        "HEAD" => "HEAD",
        "PUT" => "PUT",
        "PATCH" => "PATCH",
        "DELETE" => "DELETE",
        "OPTIONS" => "OPTIONS",
        _ => "OTHER",
    }
}

fn next_request_id() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(1);
    static INSTANCE: OnceLock<String> = OnceLock::new();
    let instance = INSTANCE.get_or_init(|| {
        let boot = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        format!("{}-{boot}", std::process::id())
    });
    format!("{}-{}", instance, NEXT.fetch_add(1, Ordering::Relaxed))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deployment_is_separate_from_the_release_tag() {
        let env = |key: &str| match key {
            "VIZ_ENV" => Some("v0.1.20".into()),
            "K_SERVICE" => Some("vizualisation-dev".into()),
            _ => None,
        };
        assert_eq!(deployment(env), "development");
        assert_eq!(deployment(|_| None), "development");
        assert_eq!(
            deployment(|k| (k == "K_SERVICE").then(|| "vizualisation-prod".into())),
            "production"
        );
        assert_eq!(
            deployment(|k| (k == "VIZ_DEPLOYMENT_ENV").then(|| "staging".into())),
            "staging"
        );
    }
}
