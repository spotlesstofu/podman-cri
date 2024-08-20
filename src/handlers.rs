use axum::{extract::Path, http::StatusCode, Json};
use tokio::net::UnixStream;

use tonic::{
    transport::{Channel, Endpoint, Uri},
    Request,
};

mod cri {
    tonic::include_proto!("runtime.v1");
}

use cri::runtime_service_client::RuntimeServiceClient;

use podman_api::models::{
    Container, ContainerCreateResponse, ContainerJson, CreateContainerConfig, IdResponse,
    ListContainer, ListPodsReport, PodRmReport, PodSpecGenerator, PodStartReport, PodStopReport,
};

/// Get a client to connect to a CRI server (for example, CRI-O).
async fn get_client() -> Result<RuntimeServiceClient<Channel>, Box<dyn std::error::Error>> {
    // We will ignore the http uri and connect to the Unix socket.
    let channel = Endpoint::try_from("http://[::]:50051")?
        .connect_with_connector(tower::service_fn(|_: Uri| {
            let path = match std::env::var("CONTAINER_RUNTIME_ENDPOINT") {
                Ok(val) => val,
                Err(err) => {
                    eprintln!(
                        "Error while reading CONTAINER_RUNTIME_ENDPOINT, using default. {err}"
                    );
                    "/run/crio/crio.sock".to_string()
                }
            };
            UnixStream::connect(path)
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

pub async fn container_list() -> Json<Vec<Container>> {
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

pub async fn container_inspect(
    Path(name): Path<String>,
) -> Result<Json<ContainerJson>, StatusCode> {
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
pub async fn container_stop() -> StatusCode {
    StatusCode::NO_CONTENT
}

pub async fn container_list_libpod() -> Json<Vec<ListContainer>> {
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
pub struct ContainerCreatePayload {
    name: String,
    body: CreateContainerConfig,
}

/// container_create_libpod responds to `POST /libpod/containers/create`.
pub async fn container_create_libpod(
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
pub async fn pod_list_libpod() -> Json<Vec<ListPodsReport>> {
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
pub async fn pod_create_libpod(Json(payload): Json<PodSpecGenerator>) -> Json<IdResponse> {
    let client = get_client();
    let pod_sandbox_config = cri::PodSandboxConfig {
        metadata: Some(cri::PodSandboxMetadata {
            name: payload.name.unwrap_or_default(),
            uid: "".to_string(),
            namespace: "".to_string(),
            attempt: 0,
        }),
        hostname: payload.hostname.unwrap_or_default(),
        log_directory: "".to_string(),
        dns_config: None,
        port_mappings: Vec::new(),
        labels: payload.labels.unwrap_or_default().into_iter().collect(),
        annotations: std::collections::HashMap::new(),
        linux: Some(cri::LinuxPodSandboxConfig {
            cgroup_parent: payload.cgroup_parent.unwrap_or_default(),
            security_context: None,
            sysctls: std::collections::HashMap::new(),
            overhead: None,
            resources: None,
        }),
        windows: None,
    };
    let message = cri::RunPodSandboxRequest {
        config: Some(pod_sandbox_config),
        runtime_handler: "runc".to_string(),
    };
    let request = Request::new(message);
    let response = client
        .await
        .unwrap()
        .run_pod_sandbox(request)
        .await
        .unwrap()
        .into_inner();
    let id = response.pod_sandbox_id;
    let response = IdResponse::new(id);
    Json(response)
}

/// pod_start_libpod responds to POST `/libpod/pods/:name/start`.
///
/// Returns a valid response but does nothing.
///
/// TODO What CRI call(s) (`rpc`) should I map this to?
pub async fn pod_start_libpod(Path(name): Path<String>) -> Json<PodStartReport> {
    let report = PodStartReport {
        id: Some(name),
        ..Default::default()
    };

    Json(report)
}

/// pod_stop_libpod responds to POST `/libpod/pods/:name/stop`.
pub async fn pod_stop_libpod(Path(name): Path<String>) -> Json<PodStopReport> {
    let client = get_client();
    let request = Request::new(cri::StopPodSandboxRequest {
        pod_sandbox_id: name.clone(),
    });
    let _response = client
        .await
        .unwrap()
        .stop_pod_sandbox(request)
        .await
        .unwrap()
        .into_inner();
    let report = PodStopReport {
        id: Some(name),
        ..Default::default()
    };
    Json(report)
}

/// pod_delete_libpod responds to DELETE `/libpod/pods/:name`.
pub async fn pod_delete_libpod(Path(name): Path<String>) -> Json<PodRmReport> {
    let client = get_client();
    let request = Request::new(cri::RemovePodSandboxRequest {
        pod_sandbox_id: name.clone(),
    });
    let _response = client
        .await
        .unwrap()
        .remove_pod_sandbox(request)
        .await
        .unwrap()
        .into_inner();
    let report = PodRmReport {
        id: Some(name),
        ..Default::default()
    };
    Json(report)
}
