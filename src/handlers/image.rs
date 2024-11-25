use axum::Json;

use tonic::Request;

use podman_api::models::{ImageCreateQueryParams, LibpodImageSummary};

use crate::cri::{self, ImageFilter};
use crate::cri_clients::get_image_client;

// POST /images/create
pub async fn image_create(Json(params): Json<ImageCreateQueryParams>) -> String {
    if params.from_image.is_some() {
        let image = params.from_image.unwrap();
        let user_specified_image = params.tag.expect("image tag or digest to pull");
        pull_image_or_local(image, user_specified_image)
            .await
            .unwrap()
    } else if params.from_src.is_some() {
        // TODO proxy to podman to build the image, then copy the image to CRI-O.
        unimplemented!("build image");
    } else {
        panic!("missing param from_image or from_src");
    }
}

async fn pull_image(image_spec: cri::ImageSpec) -> Result<String, tonic::Status> {
    let client = get_image_client().await;

    let message = cri::PullImageRequest {
        image: Some(image_spec),
        auth: None,
        sandbox_config: None,
    };
    let request = Request::new(message);

    let response = client.unwrap().pull_image(request).await?;
    let image_ref = response.into_inner().image_ref;

    Ok(image_ref)
}

async fn get_local_image(image_spec: cri::ImageSpec) -> Result<String, tonic::Status> {
    let filter = cri::ImageFilter {
        image: Some(image_spec),
    };

    let images = list_images(Some(filter)).await;

    match images.first() {
        Some(image) => Ok(image.id.clone()),
        None => Err(tonic::Status::not_found("no such image")),
    }
}

async fn pull_image_or_local(
    image: String,
    user_specified_image: String,
) -> Result<String, tonic::Status> {
    let image_spec = cri::ImageSpec {
        image,
        user_specified_image,
        ..Default::default()
    };

    let local_result = get_local_image(image_spec.clone()).await;

    if image_spec.user_specified_image.starts_with("sha") && local_result.is_ok() {
        return local_result;
    }

    let pull_result = pull_image(image_spec).await;

    if pull_result.is_ok() {
        return pull_result;
    }

    if local_result.is_ok() {
        tracing::debug!("couldn't pull image, using local");
        local_result
    } else {
        pull_result
    }
}

impl From<cri::Image> for LibpodImageSummary {
    fn from(value: cri::Image) -> Self {
        let digest = value.repo_digests[0].split_once("@").unwrap().1;

        LibpodImageSummary {
            digest: Some(digest.to_string()),
            id: Some(value.id),
            repo_tags: Some(value.repo_tags),
            repo_digests: Some(value.repo_digests),
            size: Some(value.size.try_into().unwrap_or_default()),
            ..Default::default()
        }
    }
}

async fn list_images(filter: Option<ImageFilter>) -> Vec<cri::Image> {
    let client = get_image_client().await;

    let message = cri::ListImagesRequest { filter };

    let request = Request::new(message);

    client
        .unwrap()
        .list_images(request)
        .await
        .unwrap()
        .into_inner()
        .images
}

pub async fn image_list_libpod() -> Json<Vec<LibpodImageSummary>> {
    let filter = None;

    let images: Vec<LibpodImageSummary> = list_images(filter)
        .await
        .into_iter()
        .map(|item: cri::Image| -> LibpodImageSummary { item.into() })
        .collect();

    Json(images)
}
