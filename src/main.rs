use axum::{
    extract::Request,
    http::HeaderValue,
    middleware::{self, Next},
    response::Response,
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

    let libpod_router = Router::new()
        // libpod containers routes
        .route("/containers/json", get(handlers::container_list_libpod))
        .route(
            "/containers/create",
            post(handlers::container_create_libpod),
        )
        // libpod pods routes
        .route("/pods/json", get(handlers::pod_list_libpod))
        .route("/pods/create", post(handlers::pod_create_libpod))
        .route("/pods/:name/start", post(handlers::pod_start_libpod))
        .route("/pods/:name/stop", post(handlers::pod_stop_libpod))
        .route("/pods/:name", delete(handlers::pod_delete_libpod));

    let app = Router::new()
        // compat containers routes
        .route("/containers/json", get(handlers::container_list))
        .route("/containers/create", post(handlers::container_create))
        .route("/containers/:name/json", get(handlers::container_inspect))
        .route("/containers/:name/start", post(handlers::container_start))
        .route("/containers/:name/stop", post(handlers::container_stop))
        // reply to ping
        .route("/_ping", get(handlers::ping))
        .route("/cri/_ping", get(handlers::ping))
        .route("/cri/version", get(handlers::version))
        // forward to podman all the image-related paths
        // CRI-O and Podman (root user) share the same storage for images,
        // so CRI-O can access any image pulled or built by Podman.
        .route("/images/*path", any(reverse_proxy))
        .route("/build", post(reverse_proxy))
        // forward to podman all the other paths we don't want to handle
        .route("/events", any(reverse_proxy))
        .route("/volumes", post(reverse_proxy))
        .route("/:api_version/libpod/_ping", any(reverse_proxy))
        .route("/:api_version/libpod/info", any(reverse_proxy))
        .route("/:api_version/libpod/build", any(reverse_proxy))
        .route("/:api_version/libpod/images/*path", any(reverse_proxy))
        // nest libpod routes
        .nest("/:api_version/libpod", libpod_router)
        // modify headers
        .layer(middleware::from_fn(modify_headers))
        //tracing
        .layer(TraceLayer::new_for_http());

    let path = std::env::var("PODMAN_CRI_ENDPOINT")
        .unwrap_or("/run/user/1000/podman/podman-cri.sock".into());

    serve(app, path).await;
}

/// modify_headers forces the `Content-Type` header to be `application/json`.
/// This makes the app more tolerant to clients using the wrong content type.
async fn modify_headers(mut request: Request, next: Next) -> Response {
    request
        .headers_mut()
        .insert("Content-Type", HeaderValue::from_static("application/json"));
    next.run(request).await
}
