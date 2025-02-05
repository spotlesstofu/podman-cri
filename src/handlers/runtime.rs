use axum::response::IntoResponse;
use futures::future;
use std::collections::HashMap;

use axum::{extract::Path, http::StatusCode, Json};
use tonic::Request;
use uuid::Uuid;

use podman_api::models::{
    Container, ContainerCreateResponse, ContainerJson, ContainerStartQueryParams,
    CreateContainerConfig, IdResponse, ListContainer, ListPodContainer, ListPodsReport, Mount,
    PodRmReport, PodSpecGenerator, PodStartReport, PodStopReport, SpecGenerator,
};

use crate::cri;
use crate::cri_clients::get_client;

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

impl From<cri::Container> for ListPodContainer {
    fn from(value: cri::Container) -> Self {
        ListPodContainer {
            id: Some(value.id.clone()),
            status: Some(value.state().as_str_name().to_string()),
            ..Default::default()
        }
    }
}

async fn list_containers(filter: Option<cri::ContainerFilter>) -> Vec<cri::Container> {
    let client = get_client();
    let message = cri::ListContainersRequest { filter };
    let request = Request::new(message);
    let response = client
        .await
        .unwrap()
        .list_containers(request)
        .await
        .unwrap();
    response.into_inner().containers
}

pub async fn container_list() -> Json<Vec<Container>> {
    let cri_containers = list_containers(None).await;
    let podman_containers: Vec<Container> = cri_containers
        .into_iter()
        .map(|item: cri::Container| -> Container { item.into() })
        .collect();
    Json(podman_containers)
}

pub async fn container_status(container_id: String) -> Option<cri::ContainerStatus> {
    let request = cri::ContainerStatusRequest {
        container_id,
        verbose: false,
    };
    let response = get_client()
        .await
        .unwrap()
        .container_status(request)
        .await
        .unwrap();
    response.into_inner().status
}

pub async fn container_inspect(
    Path(name): Path<String>,
) -> Result<Json<ContainerJson>, StatusCode> {
    let status = container_status(name).await;

    match status {
        Some(status) => {
            let container: ContainerJson = status.into();
            Ok(Json(container))
        }
        None => Err(StatusCode::NOT_FOUND),
    }
}

pub struct TonicStatusWrapper {
    status: tonic::Status,
}

impl From<tonic::Status> for TonicStatusWrapper {
    fn from(status: tonic::Status) -> Self {
        Self { status }
    }
}

impl IntoResponse for TonicStatusWrapper {
    fn into_response(self) -> axum::response::Response {
        let message = self.status.message().to_owned();
        axum::response::Response::new(message.into())
    }
}

async fn start_container(container_id: String) -> Result<(), tonic::Status> {
    let client = get_client();
    let request = cri::StartContainerRequest { container_id };
    client.await.unwrap().start_container(request).await?;
    Ok(())
}

pub async fn container_start(Path(name): Path<String>) -> Result<StatusCode, TonicStatusWrapper> {
    start_container(name).await?;

    Ok(StatusCode::NO_CONTENT)
}

// POST
pub async fn container_stop() -> StatusCode {
    StatusCode::NO_CONTENT
}

pub async fn container_list_libpod() -> Json<Vec<ListContainer>> {
    let cri_containers = list_containers(None).await;
    let podman_containers: Vec<ListContainer> = cri_containers
        .into_iter()
        .map(|item: cri::Container| -> ListContainer { item.into() })
        .collect();
    Json(podman_containers)
}

impl From<Mount> for cri::Mount {
    fn from(value: Mount) -> Self {
        cri::Mount {
            host_path: value.source.expect("mount source"),
            container_path: value.target.expect("mount target"),
            readonly: value.read_only.unwrap_or(false),
            ..Default::default()
        }
    }
}

impl From<String> for cri::KeyValue {
    fn from(env: String) -> Self {
        let (key, value) = env.split_once('=').expect("env key/value delimiter");
        cri::KeyValue {
            key: key.to_string(),
            value: value.to_string(),
        }
    }
}

async fn get_sandbox_config(pod_sandbox_id: String) -> cri::PodSandboxConfig {
    let filter = cri::PodSandboxFilter {
        id: pod_sandbox_id,
        ..Default::default()
    };

    let pod_sandbox_list = list_pod_sandbox(Some(filter)).await;
    let pod_sandbox = pod_sandbox_list.first().expect("pod_sandbox");

    cri::PodSandboxConfig {
        metadata: pod_sandbox.metadata.clone(),
        ..Default::default()
    }
}

async fn create_container(
    config: cri::ContainerConfig,
    pod_sandbox_id: String,
) -> (StatusCode, Json<ContainerCreateResponse>) {
    let client = get_client();

    // the CRI requires the sandbox config to be passed in the request "for easy reference" :shrug:
    let sandbox_config = get_sandbox_config(pod_sandbox_id.clone()).await;

    let message = cri::CreateContainerRequest {
        pod_sandbox_id,
        config: Some(config),
        sandbox_config: Some(sandbox_config),
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
    let response = ContainerCreateResponse { id, warnings };

    (StatusCode::CREATED, Json(response))
}

/// Cleans input from Podman Desktop.
/// Podman Desktop sometimes passes a garbage "sha256:" at the beginning of the image ID.
fn clean_image(image: String) -> String {
    match image.split_once(":") {
        Some((s1, s2)) => {
            if s1.starts_with("sha") {
                s2.to_string()
            } else {
                image
            }
        }
        None => image,
    }
}

impl From<CreateContainerConfig> for cri::ContainerConfig {
    fn from(value: CreateContainerConfig) -> Self {
        let metadata = cri::ContainerMetadata {
            name: value.name.unwrap_or_else(get_random_string),
            ..Default::default()
        };

        let image = clean_image(value.image.expect("image"));

        let image_spec = cri::ImageSpec {
            image,
            ..Default::default()
        };

        cri::ContainerConfig {
            metadata: Some(metadata),
            image: Some(image_spec),
            command: value.entrypoint.unwrap_or_default(),
            args: value.cmd.unwrap_or_default(),
            working_dir: value.working_dir.unwrap_or_default(),
            envs: value
                .env
                .unwrap_or_default()
                .into_iter()
                .map(|item| -> cri::KeyValue { item.into() })
                .collect(),
            labels: value.labels.unwrap_or_default(),
            ..Default::default()
        }
    }
}

// POST /containers/create
pub async fn container_create(
    Json(params): Json<CreateContainerConfig>,
) -> (StatusCode, Json<ContainerCreateResponse>) {
    let pod_sandbox_id = create_pod_default("").await;
    let config: cri::ContainerConfig = params.into();

    create_container(config, pod_sandbox_id).await
}

impl From<SpecGenerator> for cri::ContainerConfig {
    fn from(value: SpecGenerator) -> Self {
        let metadata = cri::ContainerMetadata {
            name: value.name.unwrap_or_else(get_random_string),
            ..Default::default()
        };

        let image = clean_image(value.image.expect("image"));

        let image_spec = cri::ImageSpec {
            image,
            ..Default::default()
        };

        cri::ContainerConfig {
            metadata: Some(metadata),
            image: Some(image_spec),
            command: value.entrypoint.unwrap_or_default(),
            args: value.command.unwrap_or_default(),
            working_dir: value.work_dir.unwrap_or("/".to_string()),
            envs: value
                .env
                .unwrap_or_default()
                .into_iter()
                .map(|(key, value)| cri::KeyValue { key, value })
                .collect(),
            mounts: value
                .mounts
                .unwrap_or_default()
                .into_iter()
                .map(|item| -> cri::Mount { item.into() })
                .collect(),
            labels: value.labels.unwrap_or_default(),
            annotations: value.annotations.unwrap_or_default(),
            ..Default::default()
        }
    }
}

// POST /libpod/containers/create
pub async fn container_create_libpod(
    Json(params): Json<SpecGenerator>,
) -> (StatusCode, Json<ContainerCreateResponse>) {
    let pod_sandbox_id = params.pod.clone().unwrap();
    let config: cri::ContainerConfig = params.into();

    create_container(config, pod_sandbox_id).await
}

async fn get_pod_containers(pod_sandbox_id: String) -> Vec<ListPodContainer> {
    let filter = cri::ContainerFilter {
        pod_sandbox_id,
        ..Default::default()
    };
    let containers = list_containers(Some(filter)).await;
    containers
        .into_iter()
        .map(|value| -> ListPodContainer { value.into() })
        .collect()
}

async fn convert_pod(pod: cri::PodSandbox) -> ListPodsReport {
    let metadata = pod.metadata.unwrap();
    let containers = get_pod_containers(pod.id.clone()).await;
    ListPodsReport {
        id: Some(pod.id),
        name: Some(metadata.name.clone()),
        namespace: Some(metadata.namespace.clone()),
        status: Some(match cri::PodSandboxState::try_from(pod.state).unwrap() {
            cri::PodSandboxState::SandboxReady => "Ready".to_string(),
            cri::PodSandboxState::SandboxNotready => "NotReady".to_string(),
        }),
        cgroup: None,
        containers: Some(containers),
        created: None,
        infra_id: Some(metadata.namespace.clone()),
        labels: Some(pod.labels),
        networks: None,
    }
}

async fn list_pod_sandbox(filter: Option<cri::PodSandboxFilter>) -> Vec<cri::PodSandbox> {
    let client = get_client();

    let request = cri::ListPodSandboxRequest { filter };
    let response = client
        .await
        .unwrap()
        .list_pod_sandbox(request)
        .await
        .unwrap();

    response.into_inner().items
}

/// pod_list_libpod responds to `GET /libpod/pods/json`.
pub async fn pod_list_libpod() -> Json<Vec<ListPodsReport>> {
    let cri_pods = list_pod_sandbox(None).await;
    let pods: Vec<ListPodsReport> = future::join_all(cri_pods.into_iter().map(convert_pod)).await;

    Json(pods)
}

fn get_random_string() -> String {
    Uuid::new_v4().to_string().split_at(8).0.to_string()
}

async fn create_pod(config: cri::PodSandboxConfig, runtime_handler: &str) -> String {
    let client = get_client();
    let message = cri::RunPodSandboxRequest {
        config: Some(config),
        runtime_handler: runtime_handler.to_string(),
    };

    let request = Request::new(message);
    let response = client
        .await
        .unwrap()
        .run_pod_sandbox(request)
        .await
        .unwrap()
        .into_inner();

    response.pod_sandbox_id
}

async fn create_pod_default(runtime_handler: &str) -> String {
    let metadata = cri::PodSandboxMetadata {
        name: get_random_string(),
        uid: get_random_string(),
        namespace: "default".to_string(),
        attempt: 0,
    };

    let config = cri::PodSandboxConfig {
        metadata: Some(metadata),
        ..Default::default()
    };
    create_pod(config, runtime_handler).await
}

/// pod_create_libpod responds to POST `/libpod/pods/create`.
pub async fn pod_create_libpod(
    Json(payload): Json<PodSpecGenerator>,
) -> (StatusCode, Json<IdResponse>) {
    let name = payload.name.unwrap_or_else(get_random_string);

    let config = cri::PodSandboxConfig {
        metadata: Some(cri::PodSandboxMetadata {
            name: name.clone(),
            uid: get_random_string(),
            namespace: payload.infra_name.unwrap_or(name.clone()),
            attempt: 0,
        }),
        hostname: payload.hostname.unwrap_or(name.clone()),
        log_directory: "/var/log/pods/".to_string(),
        port_mappings: Vec::new(),
        labels: payload.labels.unwrap_or_default(),
        annotations: HashMap::new(),
        ..Default::default()
    };

    let id = create_pod(config, "").await;
    let response = IdResponse::new(id);

    (StatusCode::CREATED, Json(response))
}

/// Start all containers in a pod.
pub async fn pod_start_libpod(Path(name): Path<String>) -> Json<PodStartReport> {
    let filter_state = cri::ContainerStateValue {
        state: cri::ContainerState::ContainerCreated.into(),
    };

    let filter = cri::ContainerFilter {
        state: Some(filter_state),
        pod_sandbox_id: name.clone(),
        ..Default::default()
    };

    let containers = list_containers(Some(filter)).await;

    let futures = containers
        .into_iter()
        .map(|container| start_container(container.id));

    let results = future::join_all(futures).await;

    // Iterate over results to collect error messages (if any).
    let error_messages: Vec<String> = results
        .into_iter()
        .filter_map(|result| match result {
            Ok(_) => None,
            Err(status) => Some(status.message().to_string()),
        })
        .collect();

    let report = PodStartReport {
        id: Some(name),
        errs: Some(error_messages),
        ..Default::default()
    };

    // TODO statuscode 409 if error_messages > 0

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

pub async fn ping() -> StatusCode {
    StatusCode::OK
}

pub async fn version() -> Json<cri::VersionResponse> {
    let client = get_client();
    let request = Request::new(cri::VersionRequest {
        version: "podman-cri".to_string(),
    });
    let response = client
        .await
        .unwrap()
        .version(request)
        .await
        .unwrap()
        .into_inner();
    Json(response)
}
