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
                    eprintln!(
                        "Error while reading CONTAINER_RUNTIME_ENDPOINT, using default. {err}"
                    );
                    "/run/crio/crio.sock".to_string()
                }
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
            created: Some(value.created_at),
            id: Some(value.id),
            image: Some(value.image_ref),
            image_id: Some(value.image_id),
            labels: Some(value.labels),
            ..Default::default()
        }
    }
}

impl From<cri::Container> for ContainerJson {
    fn from(value: cri::Container) -> Self {
        ContainerJson {
            ..Default::default()
        }
    }
}

impl From<cri::Container> for ListContainer {
    fn from(container: cri::Container) -> Self {
        ListContainer {
            id: Some(container.id),
            image: Some(container.image_ref),
            image_id: Some(container.image_id),
            created: chrono::DateTime::from_timestamp(container.created_at / 1_000_000, 0),
            created_at: Some(container.created_at.to_string()),
            state: Some(
                cri::ContainerState::try_from(container.state)
                    .unwrap()
                    .as_str_name()
                    .to_lowercase()
                    .replace("_", " "),
            ),
            labels: Some(container.labels),
            ..Default::default()
        }
    }
}

async fn container_list() -> Json<Vec<Container>> {
    let client = get_client();
    let request = Request::new(cri::ListContainersRequest::default());
    let response = client
        .await
        .unwrap()
        .list_containers(request)
        .await
        .unwrap();
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
    let response = client
        .await
        .unwrap()
        .list_containers(request)
        .await
        .unwrap();
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
    let client = get_client();
    let request = Request::new(cri::ListContainersRequest::default());
    let response = client
        .await
        .unwrap()
        .list_containers(request)
        .await
        .unwrap();
    let cri_containers = response.into_inner().containers;
    let podman_containers: Vec<ListContainer> = cri_containers
        .into_iter()
        .map(|item: cri::Container| -> ListContainer { item.into() })
        .collect();
    Json(podman_containers)
}

#[derive(serde::Deserialize)]
struct ContainerCreatePayload {
    name: String,
    body: CreateContainerConfig,
}

/// container_create_libpod responds to `POST /libpod/containers/create`.
async fn container_create_libpod(
    Json(payload): Json<ContainerCreatePayload>,
) -> Json<ContainerCreateResponse> {
    let client = get_client();
    let message = cri::CreateContainerRequest {
        pod_sandbox_id: "default".to_string(), // Assuming a default pod sandbox ID
        config: Some(cri::ContainerConfig {
            metadata: None,
            image: Some(cri::ImageSpec {
                // Assuming the image name is the same as the container name
                image: payload.name.clone(),
                ..Default::default()
            }),
            command: payload.body.cmd.unwrap_or_default(),
            args: payload.body.entrypoint.unwrap_or_default(),
            working_dir: payload.body.working_dir.unwrap_or_default(),
            envs: payload
                .body
                .env
                .unwrap_or_default()
                .into_iter()
                .map(|env| cri::KeyValue {
                    key: env.clone(),
                    value: env,
                })
                .collect(),
            mounts: Vec::new(),
            devices: Vec::new(),
            labels: payload
                .body
                .labels
                .unwrap_or_default()
                .into_iter()
                .map(|(key, value)| (key, value))
                .collect(),
            annotations: std::collections::HashMap::new(),
            log_path: format!("{}-log.log", payload.name),
            stdin: payload.body.open_stdin.unwrap_or(false),
            stdin_once: payload.body.stdin_once.unwrap_or(false),
            tty: payload.body.tty.unwrap_or(false),
            linux: None,
            windows: None,
            cdi_devices: Vec::new(),
        }),
        sandbox_config: None,
    };
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

/// pod_list_libpod responds to `GET /libpod/pods/json`.
async fn pod_list_libpod() -> Json<Vec<ListPodsReport>> {
    let client = get_client();
    let request = cri::ListPodSandboxRequest::default();
    let response = client
        .await
        .unwrap()
        .list_pod_sandbox(request)
        .await
        .unwrap();
    let cri_pods = response.into_inner().items;

    let podman_pods: Vec<ListPodsReport> = cri_pods
        .into_iter()
        .map(|pod_sandbox| {
            let mut pod_report = ListPodsReport::new();
            pod_report.id = Some(pod_sandbox.id);
            pod_report.name = Some(
                pod_sandbox
                    .metadata
                    .as_ref()
                    .map(|m| m.name.clone())
                    .unwrap_or_default(),
            );
            pod_report.namespace = Some(
                pod_sandbox
                    .metadata
                    .as_ref()
                    .map(|m| m.namespace.clone())
                    .unwrap_or_default(),
            );
            pod_report.status = Some(
                match cri::PodSandboxState::try_from(pod_sandbox.state).unwrap() {
                    cri::PodSandboxState::SandboxReady => "Ready".to_string(),
                    cri::PodSandboxState::SandboxNotready => "NotReady".to_string(),
                },
            );
            pod_report
        })
        .collect();

    Json(podman_pods)
}

/// pod_create_libpod responds to POST `/libpod/pods/create`.
async fn pod_create_libpod() -> Json<IdResponse> {
    let client = get_client();
    // TODO let message =
    let id = "".to_string();
    let response = IdResponse::new(id);
    Json(response)
}

/// pod_start_libpod responds to POST `/libpod/pods/:name/start`.
async fn pod_start_libpod() -> Json<PodStartReport> {
    Json(PodStartReport::new())
}

/// pod_stop_libpod responds to POST `/libpod/pods/:name/stop`.
async fn pod_stop_libpod() -> Json<PodStopReport> {
    Json(PodStopReport::new())
}

/// pod_delete_libpod responds to DELETE `/libpod/pods/:name`.
async fn pod_delete_libpod() -> Json<PodRmReport> {
    Json(PodRmReport::new())
}
