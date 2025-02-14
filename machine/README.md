# Podman machine

Extend the Podman machine to test and demo this project.

## Requirements

```sh
dnf install podman-machine
flatpak install io.podman_desktop.PodmanDesktop
```

## Init machine

```sh
podman machine init --rootful --now
podman machine os apply --restart 'quay.io/spotlesstofu/podman-cri:5.3'
```

Alternatively, build and push the Containerfile yourself and use that.

## Verify

Ping podman-cri on the Podman socket. You should get a `200 OK`:
```
curl -I --unix-socket /run/user/1000/podman/podman-machine-default-api.sock http://example.com/cri/_ping
```

## Proof-of-concept setup

Now you can start Podman desktop, its requests will go through podman-cri.

To force the AI extension to use this setup with the machine, there are two options.

Either:

- Change the permissions for the Podman desktop flatpak. Replace `xdg-run/podman:create` with `xdg-run/podman/podman-machine-default-api.sock:create`. You may use the Flatseal app to do this change.

-
    ```
    sudo chown root: /run/user/1000/podman/podman.sock
    ```
## Peer pods

> [!IMPORTANT]
> Below is just notes. Not tested yet.

Run the cloud API adaptor (**CAA**) inside the machine:
```sh
podman run -ti --rm \
--entrypoint /usr/local/bin/cloud-api-adaptor \
--env-file caa.env \
-v /run/peerpods:/run/peerpods \
quay.io/confidential-containers/cloud-api-adaptor:v0.8.2-amd64 \
azure \
-disable-cvm \
-subscriptionid "${AZURE_SUBSCRIPTION_ID}" \
-region "${AZURE_REGION}" \

-instance-size "${AZURE_INSTANCE_SIZE}" \
-resourcegroup "${AZURE_RESOURCE_GROUP}" \
-vxlan-port 8472 \
-subnetid "${AZURE_SUBNET_ID}" \
-securitygroupid "${AZURE_NSG_ID}" \
-imageid "${AZURE_IMAGE_ID}" \
-disable-cvm
```
