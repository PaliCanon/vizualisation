//! This service's server: renders the pages, serves the WASM bundle and the
//! assets. Run with `VIZ_GATE=off cargo leptos watch` from the repository root
//! (README.md: port 3301). The configuration is read once at boot
//! ([`vizualisation::boot::Config`]); a bad one stops the service before it
//! listens, naming the variable, never its value. The data files are read
//! from `VIZ_DATA_DIR` (`public` when unset): one missing or unreadable stops
//! the service too, naming the file.

#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    use leptos::prelude::get_configuration;
    use vizualisation::boot::Config;
    use vizualisation::data::{DATA_VAR, Data};
    use vizualisation::router::router;

    let config = match Config::from_lookup(|name| std::env::var(name).ok()) {
        Ok(config) => config,
        Err(err) => {
            eprintln!("vizualisation: {err}");
            std::process::exit(2);
        }
    };
    for line in config.log_lines() {
        println!("{line}");
    }
    println!(
        "{}",
        vizualisation::release::ReleaseInfo::server().log_line()
    );
    let dir = std::env::var(DATA_VAR).unwrap_or_else(|_| "public".into());
    let data = match Data::load(std::path::Path::new(&dir)) {
        Ok(data) => std::sync::Arc::new(data),
        Err(err) => {
            eprintln!("vizualisation: {err}");
            std::process::exit(2);
        }
    };
    println!("data: {dir}");

    let conf = get_configuration(None).expect("leptos configuration");
    let options = conf.leptos_options;
    let addr = options.site_addr;
    let app = router(options, config, data);

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("bind this service's address");
    println!("vizualisation on http://{addr}");
    axum::serve(listener, app.into_make_service())
        .await
        .expect("serve this service");
}

#[cfg(not(feature = "ssr"))]
fn main() {}
