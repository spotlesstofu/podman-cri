const podmanDesktopApi = require('@podman-desktop/api');

function activate() {
    tag = extensionApi.version
    image = `quay.io/spotlesstofu/podman-cri:${tag}`
    command = "podman"
    args = ["machine", "os", "apply", "--restart", image]
    extensionApi.process.exec(command, args)
}
