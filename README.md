# Podman CRI

This project is an attempt to create a Podman-like API to control CRI-O.
This will enable any client compatible with the Podman API to control CRI-O, or possibly any other CRI-compatible runtime.

```
Podman API client ---( Podman API )---> This project ---( CRI API )---> CRI-O
```

Only a selection of the API endpoints will be available.

Image build will be supported by proxying requests to Podman and then copying the built image to CRI-O.

## Podman API

What is the Podman API? See https://docs.podman.io/en/latest/_static/api.html.

Podman usually listens on:
```
/run/user/1000/podman/podman.sock
```

OpenAPI source:
- https://storage.googleapis.com/libpod-master-releases/swagger-latest.yaml

Endpoints to implement in this project:
```
GET /containers/{id}/json
DELETE /libpod/pods/{id}
GET /containers/json?all=true
GET /events
GET /_ping
GET /libpod/containers/json?all=true
GET /libpod/images/json
GET /libpod/info
GET /libpod/pods/json
GET /volumes
POST /build?dockerfile=&labels=
POST /containers/{id}/stop?abortSignal=
POST /libpod/containers/create
POST /libpod/pods/{id}/start
POST /libpod/pods/{id}/stop
POST /libpod/pods/create
```

## CRI API

What is the CRI API? See https://kubernetes.io/docs/concepts/architecture/cri/.

CRI-O usually listens on:
```
/run/crio/crio.sock
```

Proto files sources:
- https://github.com/kubernetes/kubernetes/blob/master/staging/src/k8s.io/cri-api/pkg/apis/runtime/v1/api.proto
- https://github.com/gogo/protobuf/blob/v1.3.2/gogoproto/gogo.proto
- https://github.com/protocolbuffers/protobuf/raw/v27.1/src/google/protobuf/descriptor.proto

# Build dependencies

- protoc
- protobuf-devel

# Development utils

Inspect Podman API calls:
```command
$ podman system info | grep podman.sock
path: /run/user/1000/podman/podman.sock
$ mv /run/user/1000/podman/podman.sock /run/user/1000/podman/podman2.sock
$ socat -t100 -v UNIX-LISTEN:/run/user/1000/podman/podman.sock,mode=777,reuseaddr,fork UNIX-CONNECT:/run/user/1000/podman/podman2.sock 2>&1 | grep -E 'GET|PUT|POST|DELETE'
```

Generate Rust code from the OpenAPI spec:
```
podman run --rm -v $(pwd):/workspace:z openapitools/openapi-generator-cli generate -i /workspace/podman-peerpods/swagger-latest.yaml -g rust-axum -o /workspace/podman-api --additional-properties=disableValidator=true,packageName=podman-api --skip-validate-spec
```

Patch models to fix missing type:
```
diff --git a/podman-api/src/models.rs b/podman-api/src/models.rs
index 1af1dde..d58b6b2 100644
--- a/src/models.rs
+++ b/src/models.rs
@@ -3,6 +3,9 @@
 use http::HeaderValue;
 use validator::Validate;

+// TODO this is a workaround for cargo error[E0412]: cannot find type `integer` in this scope
+/// synonym of u32
+type integer = u32;

 #[cfg(feature = "server")]
 use crate::header;
```

# Podman machine

See [machine/README.md](machine/README.md).
