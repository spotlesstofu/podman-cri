# Podman machine

Add functionality to the Podman machine to demo this project with peer pods.

## Container

```
cd machine/
podman build .
podman machine init --now --image ...
```

## Manually

Get a shell into the machine:
```bash
podman machine ssh
```

Install CRI-O and reboot:
```bash
rpm-ostree install cri-o containernetworking-plugins kata-containers
systemctl enable crio
systemctl reboot
```

Alternatively, get the Kata binaries from upstream:
```command
podman image pull "quay.io/kata-containers/kata-deploy:3.5.0"
podman unshare
cd $(podman image mount "quay.io/kata-containers/kata-deploy:3.5.0")
cp opt/kata-artifacts/opt/kata/bin/kata-runtime /usr/bin/
```

Run Kata:
```command
podman run -ti --rm --runtime $(pwd)/kata-runtime --runtime-flag=config=~/code/podman-peerpods/kata-remote.toml fedora-minimal
```

You may want to create a systemd unit for that last one.

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
