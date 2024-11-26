use axum::{
    routing::{any, delete, get, post},
    Router,
};

use tower_http::trace::TraceLayer;

pub mod cri {
    tonic::include_proto!("runtime.v1");
}

pub mod unix;
use crate::unix::serve;

pub mod proxy;
use crate::proxy::reverse_proxy;

pub mod cri_clients;
pub mod handlers;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        // .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let app = Router::new()
        // compat containers routes
        .route("/containers/json", get(handlers::runtime::container_list))
        .route(
            "/containers/create",
            post(handlers::runtime::container_create),
        )
        .route(
            "/containers/:name/json",
            get(handlers::runtime::container_inspect),
        )
        .route(
            "/containers/:name/start",
            post(handlers::runtime::container_start),
        )
        .route(
            "/containers/:name/stop",
            post(handlers::runtime::container_stop),
        )
        // libpod containers routes
        .route(
            "/v4.2.0/libpod/containers/json",
            get(handlers::runtime::container_list_libpod),
        )
        .route(
            "/v4.2.0/libpod/containers/create",
            post(handlers::runtime::container_create_libpod),
        )
        // libpod pods routes
        .route(
            "/v4.2.0/libpod/pods/json",
            get(handlers::runtime::pod_list_libpod),
        )
        .route(
            "/v4.2.0/libpod/pods/create",
            post(handlers::runtime::pod_create_libpod),
        )
        .route(
            "/v4.2.0/libpod/pods/:name/start",
            post(handlers::runtime::pod_start_libpod),
        )
        .route(
            "/v4.2.0/libpod/pods/:name/stop",
            post(handlers::runtime::pod_stop_libpod),
        )
        .route(
            "/v4.2.0/libpod/pods/:name",
            delete(handlers::runtime::pod_delete_libpod),
        )
        // reply to ping
        .route("/_ping", get(handlers::runtime::ping))
        .route("/cri/_ping", get(handlers::runtime::ping))
        .route("/cri/version", get(handlers::runtime::version))
        // forward to podman all the image-related paths
        // CRI-O and Podman (root user) share the same storage for images,
        // so CRI-O can access any image pulled or built by Podman.
        .route("/images/*path", any(reverse_proxy))
        .route("/v4.2.0/libpod/images/*path", get(any(reverse_proxy)))
        .route("/build", post(reverse_proxy))
        // forward to podman all the other paths we don't want to handle
        .route("/v4.2.0/libpod/info", any(reverse_proxy))
        .route("/events", any(reverse_proxy))
        .route("/volumes", post(reverse_proxy))
        // tracing
        .layer(TraceLayer::new_for_http());

    let path = std::env::var("PODMAN_CRI_ENDPOINT")
        .unwrap_or("/run/user/1000/podman/podman-cri.sock".into());

    serve(app, path).await;
}
