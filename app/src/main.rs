//! This service's server: renders the pages, serves the WASM bundle and the
//! assets. Run with `VIZ_GATE=off cargo leptos watch` from the repository root
//! (README.md: port 3301). The configuration is read once at boot
//! ([`vizualisation::boot::Config`]); a bad one stops the service before it
//! listens, naming the variable, never its value.

#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    use leptos::prelude::get_configuration;
    use vizualisation::boot::Config;
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

    let conf = get_configuration(None).expect("leptos configuration");
    let options = conf.leptos_options;
    let addr = options.site_addr;
    let app = router(options, config);

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
