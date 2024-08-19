use axum::{
    extract::Path,
    http::StatusCode,
    routing::{delete, get, post},
    Json, Router,
};

use tonic::{transport::Channel, Request};

use podman_api::models::{
    Container, ContainerCreateResponse, ContainerJson, CreateContainerConfig, IdResponse,
    ListContainer, ListPodsReport, PodRmReport, PodStartReport, PodStopReport,
};

mod cri {
    tonic::include_proto!("runtime.v1");
}

use cri::runtime_service_client::RuntimeServiceClient;

pub mod proxy;
use crate::proxy::reverse_proxy;

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

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

/// Get a client to connect to a CRI server (for example, CRI-O).
async fn get_client() -> Result<RuntimeServiceClient<Channel>, Box<dyn std::error::Error>> {
    // We will ignore the http uri and connect to the Unix socket.
    let channel = tonic::transport::Endpoint::try_from("http://[::]:50051")?
        .connect_with_connector(tower::service_fn(|_: tonic::transport::Uri| {
            let path = match std::env::var("CONTAINER_RUNTIME_ENDPOINT") {
                Ok(val) => val,
                Err(err) => {
                    eprintln!("Error while reading CONTAINER_RUNTIME_ENDPOINT, using default. {err}");
                    "/run/crio/crio.sock".to_string()
                },
            };
            tokio::net::UnixStream::connect(path)
        }))
        .await?;

    let client = RuntimeServiceClient::new(channel);
    Ok(client)
}

impl From<cri::Container> for Container {
    fn from(value: cri::Container) -> Self {
        Container {
            command: None,
            config: None,
            created: Some(value.created_at),
            default_read_only_non_recursive: None,
            host_config: None,
            id: Some(value.id),
            image: Some(value.image_ref),
            image_id: Some(value.image_id),
            labels: Some(value.labels),
            mounts: None,
            name: None,
            names: None,
            network_settings: None,
            networking_config: None,
            platform: None,
            ports: None,
            size_root_fs: None,
            size_rw: None,
            state: None,
            status: None,
        }
    }
}

impl From<cri::Container> for ContainerJson {
    fn from(value: cri::Container) -> Self {
        ContainerJson {
            app_armor_profile: None,
            args: None,
            config: None,
            created: None,
            driver: None,
            exec_ids: None,
            graph_driver: None,
            host_config: None,
            hostname_path: None,
            hosts_path: None,
            id: None,
            image: None,
            log_path: None,
            mount_label: None,
            mounts: None,
            name: None,
            network_settings: None,
            node: None,
            path: None,
            platform: None,
            process_label: None,
            resolv_conf_path: None,
            restart_count: None,
            size_root_fs: None,
            size_rw: None,
            state: None,
        }
    }
}

async fn container_list() -> Json<Vec<Container>> {
    let client = get_client();
    let request = Request::new(cri::ListContainersRequest::default());
    let response = client.await.unwrap().list_containers(request).await.unwrap();
    let cri_containers = response.into_inner().containers;
    let podman_containers: Vec<Container> = cri_containers
        .into_iter()
        .map(|item: cri::Container| -> Container { item.into() })
        .collect();
    Json(podman_containers)
}

async fn container_inspect(Path(name): Path<String>) -> Result<Json<ContainerJson>, StatusCode> {
    let client = get_client();
    let filter = cri::ContainerFilter {
        id: name,
        ..Default::default()
    };
    let message = cri::ListContainersRequest {
        filter: Some(filter),
    };
    let request = Request::new(message);
    let response = client.await.unwrap().list_containers(request).await.unwrap();
    let cri_container: Option<cri::Container> = response.into_inner().containers.pop();
    match cri_container {
        Some(cri_container) => {
            let podman_container: ContainerJson = cri_container.into();
            Ok(Json(podman_container))
        }
        None => Err(StatusCode::NOT_FOUND),
    }
}

// POST
async fn container_stop() -> StatusCode {
    StatusCode::NO_CONTENT
}

async fn container_list_libpod() -> Json<Vec<ListContainer>> {
    Json(vec![ListContainer::new()])
}

#[derive(serde::Deserialize)]
struct ContainerCreatePayload {
    name: String,
    body: CreateContainerConfig,
}

/// POST /libpod/containers/create
async fn container_create_libpod(
    Json(payload): Json<ContainerCreatePayload>,
) -> Json<ContainerCreateResponse> {
    let client = get_client();
    let message = cri::CreateContainerRequest {
        // TODO
        ..Default::default()
    };
    // TODO
    let request = Request::new(message);
    let response = client
        .await
        .unwrap()
        .create_container(request)
        .await
        .unwrap()
        .into_inner();
    let id = response.container_id;
    let warnings = Vec::new();
    let response = ContainerCreateResponse::new(id, warnings);
    Json(response)
}

async fn pod_list_libpod() -> Json<Vec<ListPodsReport>> {
    Json(vec![ListPodsReport::new()])
}

// POST
async fn pod_create_libpod() -> Json<IdResponse> {
    let client = get_client();
    // TODO let message =
    let id = "".to_string();
    let response = IdResponse::new(id);
    Json(response)
}

// POST
async fn pod_start_libpod() -> Json<PodStartReport> {
    Json(PodStartReport::new())
}

// POST
async fn pod_stop_libpod() -> Json<PodStopReport> {
    Json(PodStopReport::new())
}

async fn pod_delete_libpod() -> Json<PodRmReport> {
    Json(PodRmReport::new())
}
