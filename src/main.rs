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
            post(handlers::runtime::container_create_libpod),
        )
        .route(
            "/containers/:name/json",
            get(handlers::runtime::container_inspect),
        )
        // .route("/containers/:name/start", post(handlers::container_start))
        .route(
            "/containers/:name/stop",
            post(handlers::runtime::container_stop),
        )
        .route("/images/create", post(handlers::image::image_create))
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
        .route(
            "/v4.2.0/libpod/images/json",
            get(handlers::image::image_list_libpod),
        )
        // reply to ping
        .route("/_ping", get(handlers::runtime::ping))
        .route("/cri/_ping", get(handlers::runtime::ping))
        // forward to podman all the paths we don't want to handle
        .route("/v4.2.0/libpod/info", any(reverse_proxy))
        .route("/events", any(reverse_proxy))
        .layer(TraceLayer::new_for_http());

    let path = std::env::var("PODMAN_CRI_ENDPOINT")
        .unwrap_or("/run/user/1000/podman/podman-cri.sock".into());

    serve(app, path).await;
}
