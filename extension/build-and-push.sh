#!/usr/bin/env bash

set -e
set -x

npm run build
podman build -t quay.io/spotlesstofu/peer-pods-extension .
podman push quay.io/spotlesstofu/peer-pods-extension:latest
