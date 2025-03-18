use axum::{
    body::{to_bytes, Body, Bytes},
    extract::Request,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use http_body_util::Full;
use hyper_util::client::legacy::Client;
use hyperlocal::{UnixClientExt, UnixConnector, Uri};

const PODMAN_ENDPOINT_DEFAULT: &str = "/run/user/1000/podman/podman.sock";

pub async fn reverse_proxy(req: Request<Body>) -> Result<Response, StatusCode> {
    let path = req.uri().path();
    let path_query = req
        .uri()
        .path_and_query()
        .map(|v| v.as_str())
        .unwrap_or(path);

    let socket = std::env::var("PODMAN_ENDPOINT").unwrap_or(PODMAN_ENDPOINT_DEFAULT.into());
    let uri = Uri::new(socket, path_query);

    let (parts, body) = req.into_parts();
    let bytes = to_bytes(body, usize::MAX).await.unwrap();

    let request: hyper::Request<Full<Bytes>> = hyper::Request::builder()
        .method(parts.method)
        .uri(uri)
        .body(Full::from(bytes))
        .expect("request builder");

    let client: Client<UnixConnector, Full<Bytes>> = Client::unix();
    let response = client
        .request(request)
        .await
        .map_err(|_| StatusCode::BAD_REQUEST)?
        .into_response();

    Ok(response)
}
