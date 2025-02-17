# peer pods extension

## Install

On Podman Desktop:

1. Open the Extensions tab.
1. Push "Install custom".
1. Fill in the "OCI image" field with the following:
    ```
    quay.io/spotlesstofu/peer-pods-extension
    ```

## Development

Setup the development environment:

  1. Clone and `cd` into the Podman Desktop repo.
  1. Run `pnpm install` to install the dependencies.

Run Podman Desktop with the peer pods extension in development mode:

```
pnpm watch --extension-folder /path/to/peer/pods/extension
```

## Build

```
npm run build
podman build -t quay.io/spotlesstofu/peer-pods-extension .
```
