# Podman machine

Extend the Podman machine to test and demo this project.

Build the container image:
```
cd machine/
podman build -t podman-machine-cri .
```

Save the container image to file:
```
podman save podman-machine-cri:latest -o image
```

Convert the container image to a disk image (see https://github.com/dustymabe/build-podman-machine-os-disks/):
```
./build-podman-machine-os-disks.sh image

```

Create and start the machine:
```
podman machine init podman-machine-cri --now --image ./podman-machine-cri.qcow2
```

Get a shell into the machine:
```bash
podman machine ssh
```

Run the cloud API adaptor (**CAA**):

```console
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

You may want to create a systemd unit for that last one.
