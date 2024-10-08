use axum::Json;

use tonic::Request;

use podman_api::models::{ImageCreateQueryParams, ImageListLibpodQueryParams, LibpodImageSummary};

use crate::cri;
use crate::cri_clients::get_image_client;

// POST /images/create
pub async fn image_create(Json(params): Json<ImageCreateQueryParams>) -> String {
    let image = params.from_image.expect("image to pull");
    let tag = params.tag.expect("image tag or digest to pull");
    image_pull(image, tag).await
}

async fn image_pull(image: String, tag: String) -> String {
    let client = get_image_client();

    let message = cri::PullImageRequest {
        image: Some(cri::ImageSpec {
            image,
            user_specified_image: tag,
            ..Default::default()
        }),
        auth: None,
        sandbox_config: None,
    };

    let request = Request::new(message);
    let response = client
        .await
        .unwrap()
        .pull_image(request)
        .await
        .unwrap()
        .into_inner();

    response.image_ref
}

impl From<cri::Image> for LibpodImageSummary {
    fn from(value: cri::Image) -> Self {
        LibpodImageSummary {
            id: Some(value.id),
            repo_tags: Some(value.repo_tags),
            repo_digests: Some(value.repo_digests),
            size: Some(value.size.try_into().unwrap_or_default()),
            ..Default::default()
        }
    }
}

pub async fn image_list_libpod(
    Json(params): Json<ImageListLibpodQueryParams>,
) -> Json<Vec<LibpodImageSummary>> {
    if params.filters.is_some() {
        tracing::debug!("ignoring filters")
    }

    let client = get_image_client();

    let message = cri::ListImagesRequest {
        ..Default::default()
    };

    let request = Request::new(message);
    let response = client
        .await
        .unwrap()
        .list_images(request)
        .await
        .unwrap()
        .into_inner();

    let images: Vec<LibpodImageSummary> = response
        .images
        .into_iter()
        .map(|item: cri::Image| -> LibpodImageSummary { item.into() })
        .collect();

    Json(images)
}
