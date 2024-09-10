use axum::{
    body::Body,
    extract::connect_info::{self},
    http::{Request, Response},
    Router,
};
use hyper::body::Incoming;
use hyper_util::{
    rt::{TokioExecutor, TokioIo},
    server,
};

use std::{convert::Infallible, path::PathBuf, sync::Arc};
use tokio::net::{unix::UCred, UnixListener, UnixStream};
use tower::Service;

pub async fn serve(app: Router, path: String) {
    let path_buf = PathBuf::from(path.clone());

    let _ = tokio::fs::remove_file(&path_buf).await;
    tokio::fs::create_dir_all(path_buf.parent().unwrap())
        .await
        .unwrap();

    let uds = UnixListener::bind(path_buf.clone()).unwrap();

    let mut make_service = app.into_make_service_with_connect_info::<UdsConnectInfo>();

    // See https://github.com/tokio-rs/axum/blob/main/examples/serve-with-hyper/src/main.rs for
    // more details about this setup
    loop {
        let (socket, _remote_addr) = uds.accept().await.unwrap();

        let tower_service = unwrap_infallible(make_service.call(&socket).await);

        tokio::spawn(async move {
            let socket = TokioIo::new(socket);

            let hyper_service = hyper::service::service_fn(move |request: Request<Incoming>| {
                tower_service.clone().call(request)
            });

            if let Err(err) = server::conn::auto::Builder::new(TokioExecutor::new())
                .serve_connection_with_upgrades(socket, hyper_service)
                .await
            {
                eprintln!("failed to serve connection: {err:#}");
            }
        });
    }
}

pub async fn send(request: Request<Body>, path: String) -> Response<Incoming> {
    let stream = TokioIo::new(UnixStream::connect(path).await.unwrap());
    let (mut sender, conn) = hyper::client::conn::http1::handshake(stream).await.unwrap();
    tokio::task::spawn(async move {
        if let Err(err) = conn.await {
            println!("Connection failed: {:?}", err);
        }
    });

    sender.send_request(request).await.unwrap()
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
struct UdsConnectInfo {
    peer_addr: Arc<tokio::net::unix::SocketAddr>,
    peer_cred: UCred,
}

impl connect_info::Connected<&UnixStream> for UdsConnectInfo {
    fn connect_info(target: &UnixStream) -> Self {
        let peer_addr = target.peer_addr().unwrap();
        let peer_cred = target.peer_cred().unwrap();

        Self {
            peer_addr: Arc::new(peer_addr),
            peer_cred,
        }
    }
}

fn unwrap_infallible<T>(result: Result<T, Infallible>) -> T {
    match result {
        Ok(value) => value,
        Err(err) => match err {},
    }
}
