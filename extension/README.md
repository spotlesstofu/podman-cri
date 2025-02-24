# peer pods extension

## Install

1. Open Podman Desktop.
1. Go to the Extensions tab.
1. Push "Install custom".
1. Fill in the "OCI image" field with the following:
    ```
    quay.io/spotlesstofu/peer-pods-extension
    ```
1. Wait for the extension to install.
1. Populate the Preferences.
1. Run the Onboarding.

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
