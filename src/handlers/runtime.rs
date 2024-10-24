use futures::future;
use std::collections::HashMap;

use axum::{extract::Path, http::StatusCode, Json};
use tonic::Request;
use uuid::Uuid;

use podman_api::models::{
    Container, ContainerCreateResponse, ContainerJson, CreateContainerConfig, IdResponse,
    ListContainer, ListPodContainer, ListPodsReport, Mount, PodRmReport, PodSpecGenerator,
    PodStartReport, PodStopReport,
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

pub async fn container_inspect(
    Path(name): Path<String>,
) -> Result<Json<ContainerJson>, StatusCode> {
    let filter = cri::ContainerFilter {
        id: name,
        ..Default::default()
    };
    let cri_container: Option<cri::Container> = list_containers(Some(filter)).await.pop();
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

impl From<CreateContainerConfig> for cri::ContainerConfig {
    fn from(value: CreateContainerConfig) -> Self {
        let image = value.image.map(|image| cri::ImageSpec {
            image,
            ..Default::default()
        });

        cri::ContainerConfig {
            image,
            command: value.entrypoint.unwrap_or_default(),
            args: value.cmd.unwrap_or_default(),
            working_dir: value.working_dir.unwrap_or_default(),
            envs: value
                .env
                .unwrap_or_default()
                .into_iter()
                .map(|env| -> cri::KeyValue { env.into() })
                .collect(),
            labels: value.labels.unwrap_or_default().into_iter().collect(),
            ..Default::default()
        }
    }
}

// POST /containers/create
// POST /libpod/containers/create
// TODO "sandbox config is nil"
pub async fn container_create_libpod(
    Json(params): Json<CreateContainerConfig>,
) -> Json<ContainerCreateResponse> {
    let client = get_client();

    let config: cri::ContainerConfig = params.into();

    // CreateContainer creates a new container in specified PodSandbox
    let message = cri::CreateContainerRequest {
        pod_sandbox_id: "default".to_string(),
        config: Some(config),
        ..Default::default()
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

    Json(response)
}

async fn get_pod_containers(pod_sandbox_id: String) -> Vec<ListPodContainer> {
    let filter = cri::ContainerFilter {
        pod_sandbox_id,
        ..Default::default()
    };
    let containers = list_containers(Some(filter)).await;
    containers.into_iter().map(|value| value.into()).collect()
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
    let pods: Vec<ListPodsReport> = future::join_all(cri_pods.into_iter().map(convert_pod)).await;

    Json(pods)
}

fn get_random_string() -> String {
    Uuid::new_v4().to_string().split_at(8).0.to_string()
}

/// pod_create_libpod responds to POST `/libpod/pods/create`.
pub async fn pod_create_libpod(
    Json(payload): Json<PodSpecGenerator>,
) -> (StatusCode, Json<IdResponse>) {
    let client = get_client();

    let name = payload.name.unwrap_or(get_random_string());

    let config = cri::PodSandboxConfig {
        metadata: Some(cri::PodSandboxMetadata {
            name: name.clone(),
            uid: "".to_string(),
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

    let message = cri::RunPodSandboxRequest {
        config: Some(config),
        runtime_handler: "".to_string(),
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

    (StatusCode::CREATED, Json(response))
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

pub async fn ping() -> StatusCode {
    StatusCode::OK
}
