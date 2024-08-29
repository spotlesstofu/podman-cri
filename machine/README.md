# Podman machine

Extend the Podman machine to test and demo this project.

## Manually

Create a Podman machine:
```
podman machine init
podman machine start
```

Get a shell into the machine:
```
podman machine ssh
```

Install dependencies and reboot:
```
sudo rpm-ostree install cri-o containernetworking-plugins kata-containers
sudo systemctl reboot
```

Copy the configuration:
```
cat kata.toml | podman machine ssh "sudo tee /opt/kata/configuration-remote.toml"
cat crio.conf | podman machine ssh "sudo tee /etc/crio/crio.conf.d/50-kata-remote"
```

Back to the machine:
```
podman machine ssh
```

Enable services:
```
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

## Scripted (incomplete)

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
./build-podman-machine-os-disks.sh ...
```

Create and start the machine:
```
podman machine init podman-machine-cri --now --image ./podman-machine-cri.qcow2
```
