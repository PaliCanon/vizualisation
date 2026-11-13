//! Operational JSON logs, usable without an OpenTelemetry backend.
//!
//! The site installs a tracing subscriber at its two entry points. The CSR
//! fragment only emits events: its host owns the subscriber. No exporter,
//! network call or persistent browser identifier is created here.

#[cfg(all(feature = "browser", target_arch = "wasm32"))]
mod browser;
#[cfg(feature = "ssr")]
pub mod server;

/// The SSR meta carrying the deployment environment to hydration.
pub const ENV_META: &str = "viz-deployment-environment";

/// Identity attached to every Map event, independently of its sink.
#[cfg(any(feature = "ssr", all(feature = "browser", target_arch = "wasm32")))]
#[derive(Clone)]
pub(crate) struct Identity {
    pub(crate) environment: String,
    pub(crate) version: String,
    pub(crate) commit: String,
    pub(crate) runtime: &'static str,
}

/// Emit only explicit, sanitized fields; never pass a user value or a raw
/// error through Debug/Display. The macro also avoids reading browser metas
/// when the host has no subscriber or the event is filtered out.
#[cfg(any(feature = "ssr", all(feature = "browser", target_arch = "wasm32")))]
macro_rules! event {
    ($level:ident, $name:literal $(, $($fields:tt)*)?) => {{
        if tracing::enabled!(target: "map", tracing::Level::$level) {
            let identity = crate::telemetry::identity();
            tracing::event!(target: "map", tracing::Level::$level,
                event.name = $name,
                severity = crate::telemetry::severity(tracing::Level::$level),
                service.namespace = "palicanon",
                service.name = "map",
                service.version = %identity.version,
                service.commit = %identity.commit,
                deployment.environment.name = %identity.environment,
                runtime = identity.runtime,
                $($($fields)*)?
            );
        }
    }};
}
#[cfg(any(feature = "ssr", all(feature = "browser", target_arch = "wasm32")))]
pub(crate) use event;

#[cfg(any(feature = "ssr", all(feature = "browser", target_arch = "wasm32")))]
pub(crate) fn severity(level: tracing::Level) -> &'static str {
    match level {
        tracing::Level::ERROR => "ERROR",
        tracing::Level::WARN => "WARNING",
        tracing::Level::INFO => "INFO",
        tracing::Level::DEBUG | tracing::Level::TRACE => "DEBUG",
    }
}

#[cfg(any(feature = "ssr", all(feature = "browser", target_arch = "wasm32")))]
pub(crate) fn identity() -> Identity {
    #[cfg(feature = "ssr")]
    return server::identity().clone();
    #[cfg(all(feature = "browser", target_arch = "wasm32", not(feature = "ssr")))]
    return browser::identity();
    #[cfg(not(any(feature = "ssr", all(feature = "browser", target_arch = "wasm32"))))]
    Identity {
        environment: "unknown".into(),
        version: "unknown".into(),
        commit: String::new(),
        runtime: "host",
    }
}

/// A panic's code location is useful; its payload may contain user data.
#[cfg(any(feature = "ssr", all(feature = "browser", target_arch = "wasm32")))]
pub(crate) fn panic(info: &std::panic::PanicHookInfo<'_>) {
    let file = info
        .location()
        .map(|location| location.file().rsplit('/').next().unwrap_or("unknown"));
    let line = info.location().map(std::panic::Location::line);
    event!(ERROR, "runtime.panic", code.file = file, code.line = line);
}

/// Installs the site's console sink. A CSR host may opt in, or install its
/// own subscriber instead. An existing subscriber is always preserved.
#[cfg(all(feature = "browser", target_arch = "wasm32"))]
pub fn init_browser() {
    browser::init();
}

/// A data label from a closed list, never a host URL or a raw path.
#[cfg(any(feature = "ssr", all(feature = "browser", target_arch = "wasm32")))]
pub(crate) fn asset(name: &str) -> &'static str {
    match name {
        "graph.json" => "graph.json",
        "graph-sim.json" => "graph-sim.json",
        "graph-sim.bin" => "graph-sim.bin",
        "translations.json" => "translations.json",
        "blurbs-en.json" => "blurbs-en.json",
        "blurbs-fr.json" => "blurbs-fr.json",
        "blurbs-vi.json" => "blurbs-vi.json",
        _ => "unknown",
    }
}

/// A bounded data failure, usable by the server and browser host adapters.
#[cfg(any(feature = "ssr", all(feature = "browser", target_arch = "wasm32")))]
pub(crate) fn data_failed(name: &str, reason: &'static str, status: Option<u16>) {
    event!(ERROR, "map.data.failed", asset = asset(name), error.type = reason, http.response.status_code = status);
}

#[cfg(all(test, feature = "ssr"))]
pub(crate) mod tests {
    use std::io::{self, Write};
    use std::sync::{Arc, Mutex};

    #[derive(Clone, Default)]
    pub(crate) struct Capture(Arc<Mutex<Vec<u8>>>);

    impl Write for Capture {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(bytes);
            Ok(bytes.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    pub(crate) fn capture(run: impl FnOnce()) -> Vec<serde_json::Value> {
        use tracing_subscriber::prelude::*;
        let output = Capture::default();
        let writer = output.clone();
        let subscriber = tracing_subscriber::registry().with(server_layer(move || writer.clone()));
        tracing::subscriber::with_default(subscriber, run);
        let bytes = output.0.lock().unwrap();
        String::from_utf8_lossy(&bytes)
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect()
    }

    use super::server::layer as server_layer;

    #[test]
    fn asset_failures_drop_unknown_names_and_keep_only_static_categories() {
        let logs = capture(|| {
            super::data_failed(
                "https://secret.invalid/private?q=canary",
                "parse",
                Some(502),
            );
            super::data_failed("graph.json", "network", None);
        });
        assert_eq!(logs.len(), 2);
        assert_eq!(logs[0]["asset"], "unknown");
        assert_eq!(logs[0]["error.type"], "parse");
        assert_eq!(logs[0]["http.response.status_code"], 502);
        assert_eq!(logs[1]["asset"], "graph.json");
        assert!(!serde_json::to_string(&logs).unwrap().contains("canary"));
    }

    #[test]
    fn json_logs_have_identity_severity_and_only_the_application_target() {
        let logs = capture(|| {
            super::event!(WARN, "test.failed", error.type = "network");
            tracing::error!(target: "http_client", "secret library error");
            tracing::error!(target: "map_dependency", "secret library error");
            super::event!(DEBUG, "test.debug");
        });
        assert_eq!(logs.len(), 1);
        let log = &logs[0];
        assert_eq!(log["event.name"], "test.failed");
        assert_eq!(log["severity"], "WARNING");
        assert_eq!(log["service.namespace"], "palicanon");
        assert_eq!(log["service.name"], "map");
        assert_eq!(log["runtime"], "server");
        assert!(log["timestamp"].as_str().is_some());
        assert!(log["service.version"].as_str().is_some());
        assert_eq!(log["error.type"], "network");
    }
}
