//! The Map server: bounded lifecycle/data logs, no configuration values or raw errors.

#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    use vizualisation::telemetry::server;
    server::init();
    if let Err(stage) = serve().await {
        server::failed(stage);
        std::process::exit(2);
    }
    server::stopped();
}

#[cfg(feature = "ssr")]
async fn serve() -> Result<(), &'static str> {
    use leptos::prelude::get_configuration;
    use vizualisation::{
        boot::{BootError, Config},
        gate::Gate,
        origin::Origin,
        router::router,
        telemetry::server,
    };
    let started = std::time::Instant::now();
    let config =
        Config::from_lookup(|name| std::env::var(name).ok()).map_err(BootError::variable)?;
    let checked = matches!(&config.origin, Origin::Checked(_));
    let password = matches!(&config.gate, Gate::Password(_));
    let shutdown = server::shutdown();
    let data = load_data()?;
    let options = get_configuration(None)
        .map_err(|_| "leptos_config")?
        .leptos_options;
    let addr = options.site_addr;
    let app = router(options, config, data);
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .map_err(|_| "listen")?;
    server::started(checked, password, started.elapsed());
    axum::serve(listener, app.into_make_service())
        .with_graceful_shutdown(shutdown)
        .await
        .map_err(|_| "serve")
}

#[cfg(feature = "ssr")]
fn load_data() -> Result<std::sync::Arc<vizualisation::data::Data>, &'static str> {
    use vizualisation::data::{DATA_VAR, Data};
    let started = std::time::Instant::now();
    let dir = std::env::var(DATA_VAR).unwrap_or_else(|_| "public".into());
    let data = Data::load(std::path::Path::new(&dir)).map_err(|error| {
        vizualisation::telemetry::server::data_failed(&error.file);
        DATA_VAR
    })?;
    vizualisation::telemetry::server::data_ready(data.raw.iter().count(), started.elapsed());
    Ok(std::sync::Arc::new(data))
}

#[cfg(not(feature = "ssr"))]
fn main() {}
