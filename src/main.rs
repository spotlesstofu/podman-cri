use axum::{
    routing::{delete, get, post},
    Router,
};

pub mod unix;
use crate::unix::serve;

pub mod proxy;
use crate::proxy::reverse_proxy;

pub mod handlers;

#[tokio::main]
async fn main() {
    let app = Router::new()
        // compat containers routes
        .route("/containers/json", get(handlers::container_list))
        .route("/containers/:name/json", get(handlers::container_inspect))
        .route("/containers/:name/stop", post(handlers::container_stop))
        // libpod containers routes
        .route(
            "/libpod/containers/json",
            get(handlers::container_list_libpod),
        )
        .route(
            "/libpod/containers/create",
            post(handlers::container_create_libpod),
        )
        // libpod pods routes
        .route("/libpod/pods/json", get(handlers::pod_list_libpod))
        .route("/libpod/pods/create", post(handlers::pod_create_libpod))
        .route("/libpod/pods/:name/start", post(handlers::pod_start_libpod))
        .route("/libpod/pods/:name/stop", post(handlers::pod_stop_libpod))
        .route("/libpod/pods/:name", delete(handlers::pod_delete_libpod))
        // reply to ping
        .route("/_ping", get(handlers::ping))
        .route("/cri/_ping", get(handlers::ping))
        // forward to podman all the non-matching paths
        .fallback(reverse_proxy);

    let path = std::env::var("PODMAN_CRI_ENDPOINT")
        .unwrap_or("/run/user/1000/podman/podman-cri.sock".into());

    serve(app, path).await;
}
