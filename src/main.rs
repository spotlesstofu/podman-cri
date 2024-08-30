use axum::{
    routing::{delete, get, post},
    Router,
};

pub mod unix;
use crate::unix::serve;

pub mod proxy;
use crate::proxy::reverse_proxy;

pub mod handlers;
use crate::handlers::{
    container_create_libpod, container_inspect, container_list, container_list_libpod,
    container_stop, pod_create_libpod, pod_delete_libpod, pod_list_libpod, pod_start_libpod,
    pod_stop_libpod,
};

#[tokio::main]
async fn main() {
    let app = Router::new()
        // compat containers routes
        .route("/containers/json", get(container_list))
        .route("/containers/:name/json", get(container_inspect))
        .route("/containers/:name/stop", post(container_stop))
        // libpod containers routes
        .route("/libpod/containers/json", get(container_list_libpod))
        .route("/libpod/containers/create", post(container_create_libpod))
        // libpod pods routes
        .route("/libpod/pods/json", get(pod_list_libpod))
        .route("/libpod/pods/create", post(pod_create_libpod))
        .route("/libpod/pods/:name/start", post(pod_start_libpod))
        .route("/libpod/pods/:name/stop", post(pod_stop_libpod))
        .route("/libpod/pods/:name", delete(pod_delete_libpod))
        // forward to podman all the non-matching paths
        .fallback(reverse_proxy);

    let path = "/run/user/1000/podman/podman.sock".to_string();

    serve(app, path).await;
}
