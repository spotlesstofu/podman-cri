use axum::{
    routing::{delete, get, post},
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

pub mod handlers;
pub mod cri_clients;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        // .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let app = Router::new()
        // compat containers routes
        .route("/containers/json", get(handlers::container_list))
        .route("/containers/:name/json", get(handlers::container_inspect))
        .route("/containers/:name/stop", post(handlers::container_stop))
        .route("/images/create", post(handlers::images_create))
        // libpod containers routes
        .route(
            "/v4.2.0/libpod/containers/json",
            get(handlers::container_list_libpod),
        )
        .route(
            "/v4.2.0/libpod/containers/create",
            post(handlers::container_create_libpod),
        )
        // libpod pods routes
        .route("/v4.2.0/libpod/pods/json", get(handlers::pod_list_libpod))
        .route(
            "/v4.2.0/libpod/pods/create",
            post(handlers::pod_create_libpod),
        )
        .route(
            "/v4.2.0/libpod/pods/:name/start",
            post(handlers::pod_start_libpod),
        )
        .route(
            "/v4.2.0/libpod/pods/:name/stop",
            post(handlers::pod_stop_libpod),
        )
        .route(
            "/v4.2.0/libpod/pods/:name",
            delete(handlers::pod_delete_libpod),
        )
        .route(
            "/v4.2.0/libpod/images/json",
            get(handlers::images_list_libpod),
        )
        // reply to ping
        .route("/_ping", get(handlers::ping))
        .route("/cri/_ping", get(handlers::ping))
        // forward to podman all the non-matching paths
        .fallback(reverse_proxy)
        .layer(TraceLayer::new_for_http());

    let path = std::env::var("PODMAN_CRI_ENDPOINT")
        .unwrap_or("/run/user/1000/podman/podman-cri.sock".into());

    serve(app, path).await;
}
