use std::error::Error;

use tokio::net::UnixStream;
use tonic::transport::{Channel, Endpoint, Uri};

use crate::cri::runtime_service_client::RuntimeServiceClient;

async fn get_channel() -> Result<Channel, Box<dyn Error>> {
    // We will ignore the http uri and connect to the Unix socket.
    let channel = Endpoint::try_from("http://[::]:50051")?
        .connect_with_connector(tower::service_fn(|_: Uri| {
            let path =
                std::env::var("CONTAINER_RUNTIME_ENDPOINT").unwrap_or("/run/crio/crio.sock".into());
            UnixStream::connect(path)
        }))
        .await?;
    Ok(channel)
}

/// Get a client to connect to a CRI server (for example, CRI-O).
pub async fn get_client() -> Result<RuntimeServiceClient<Channel>, Box<dyn Error>> {
    let channel = get_channel().await?;
    let client = RuntimeServiceClient::new(channel);
    Ok(client)
}
