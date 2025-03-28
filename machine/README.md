# Podman machine

Manual setup of the Podman machine. For development only.

## Requirements

```sh
dnf install podman-machine
flatpak install io.podman_desktop.PodmanDesktop
```

## Init machine

```sh
podman machine init --rootful --now
podman machine os apply --restart 'quay.io/spotlesstofu/podman-cri:5.5'
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
