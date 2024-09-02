# Podman machine

Extend the Podman machine to test and demo this project.

## Quick start

```sh
podman machine init --now
podman machine os apply --restart 'quay.io/spotlesstofu/podman-cri:5.1'
```

## Manually

Create a Podman machine:
```sh
podman machine init
podman machine start
```

Get a shell into the machine:
```sh
podman machine ssh
```

Install dependencies and reboot:
```sh
sudo rpm-ostree install cri-o containernetworking-plugins kata-containers
sudo systemctl reboot
```

Copy the configuration:
```sh
cat kata.toml | podman machine ssh "sudo tee /opt/kata/configuration-remote.toml"
cat crio.conf | podman machine ssh "sudo tee /etc/crio/crio.conf.d/50-kata-remote"
```

Back to the machine:
```sh
podman machine ssh
```

Enable services:
```sh
systemctl enable --now crio
```

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
