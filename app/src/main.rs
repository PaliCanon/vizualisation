//! This service's server: renders the pages, serves the WASM bundle and the
//! assets. Run with `cargo leptos watch` from the repository root (README.md:
//! port 3301). The boot configuration and the gates come in slice S3.

#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    use leptos::prelude::get_configuration;
    use leptos_axum::{LeptosRoutes, file_and_error_handler, generate_route_list};
    use vizualisation::app::{App, shell};

    let conf = get_configuration(None).expect("leptos configuration");
    let options = conf.leptos_options;
    let addr = options.site_addr;
    let routes = generate_route_list(App);
    let app = axum::Router::new()
        .leptos_routes(&options, routes, {
            let options = options.clone();
            move || shell(options.clone())
        })
        .fallback(file_and_error_handler(shell))
        .with_state(options);

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
