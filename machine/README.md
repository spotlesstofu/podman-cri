# Podman machine

Extend the Podman machine to test and demo this project.

## Requirements

```sh
dnf install podman-machine
flatpak install io.podman_desktop.PodmanDesktop
```

## Init machine

```sh
podman machine init --now
podman machine os apply --restart 'quay.io/spotlesstofu/podman-cri:5.1'
```

Alternatively, build and push the Containerfile yourself and use that.

## Proof-of-concept setup

A vast selection of hacks allows to get things going,
until we have a Podman desktop extension to sort things out.

Start the machine:
```
podman machine start
```

Copy the binary into the machine:
```
cat target/debug/podman-cri | podman machine ssh --username root "cat > podman-cri"
```

Enter the machine:
```
podman machine ssh --username root
```

Replace the Podman socket:
```
mv /run/user/1000/podman/podman.sock /run/user/1000/podman/podman2.sock
```

Start podman-cri, make it listen on the Podman socket:
```
PODMAN_ENDPOINT=/run/podman/podman.sock \
PODMAN_CRI_ENDPOINT=/run/user/1000/podman/podman.sock \
./podman-cri
```

```
chown core /run/user/1000/podman/podman.sock
```

Back to your host, ping podman-cri on the Podman socket. You should get a `200 OK`:
```
curl -I --unix-socket /run/user/1000/podman/podman-machine-default-api.sock http://example.com/cri/_ping
```

Now you can start Podman desktop, its requests will go through podman-cri.

To force the AI extension to use this setup, change the permissions for the Podman desktop flatpak. Replace `xdg-run/podman:create` with `xdg-run/podman/podman-machine-default-api.sock:create`. You may use the Flatseal app to do this change.

## Peer pods

Run the cloud API adaptor (**CAA**) inside the machine:
```sh
podman run -ti --rm \
--entrypoint /usr/local/bin/cloud-api-adaptor \
--env-file caa.env \
-v /run/peerpods:/run/peerpods \
quay.io/confidential-containers/cloud-api-adaptor:v0.8.2-amd64 azure \
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
