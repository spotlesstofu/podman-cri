use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use hyper_util::client::legacy::Client;
use http_body_util::Full;
use hyper::body::Bytes;
use hyper::Request;
use hyperlocal::{UnixClientExt, UnixConnector, Uri};

pub async fn reverse_proxy(mut req: axum::extract::Request<axum::body::Bytes>) -> Result<Response, StatusCode> {
    let path = req.uri().path();
    let path_query = req
        .uri()
        .path_and_query()
        .map(|v| v.as_str())
        .unwrap_or(path);

    let uri = Uri::new("/run/user/1000/podman/podman.sock", path_query);
    let client: Client<UnixConnector, Full<Bytes>> = Client::unix();

    *req.uri_mut() = axum::http::uri::Uri::try_from(uri).unwrap();

    let (parts, body) = req.into_parts();

    let req: Request<Full<Bytes>> = Request::builder()
    .method(parts.method)
    .uri(parts.uri)
    .body(Full::from(body))
    .expect("request builder");

    // todo: add parts.headers to req?

    Ok(client
        .request(req)
        .await
        .map_err(|_| StatusCode::BAD_REQUEST)?
        .into_response())
}
