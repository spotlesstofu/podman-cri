import type { ExtensionContext } from '@podman-desktop/api';
import * as extensionApi from '@podman-desktop/api';

const machineImage = "quay.io/spotlesstofu/podman-cri:5.5"
const caaImage = "quay.io/confidential-containers/cloud-api-adaptor:v0.8.2-amd64"

async function execPodman(args) {
  const command = "podman";
  return await extensionApi.process.exec(command, args)
}

const setupMachine = extensionApi.commands.registerCommand('peerpods.onboarding.setupMachine', async () => {
  // machine init
  try {
    await execPodman(["machine", "init", "--rootful", "--now"]);
  } catch (e) {
    if (e.stderr.includes("already exists")) {
      // pass, no need to create the machine
    } else {
      console.error(e.stderr);
      throw e;
    }
  }

  // TODO await machine is up

  // machine os apply
  try {
    await execPodman(["machine", "os", "apply", "--restart", machineImage]);
  } catch (e) {
    if (e.stderr.includes("refs are equal")) {
      // pass, image already applied
    } else {
      console.error(e.stderr);
      throw e;
    }
  }
})

const startPeerpods = extensionApi.commands.registerCommand('peerpods.onboarding.startPeerpods', async () => {
  await execPodman(["machine", "ssh", "--username", "root", "mkdir -p /run/peerpod && chown core: /run/peerpod"])

  const peerpodsConfiguration = extensionApi.configuration.getConfiguration("peerpods")
  const envFilePath = await peerpodsConfiguration.get("envFilePath")
  const envFiles: string[] = []
  if (typeof envFilePath === "string") {
    envFiles.push(envFilePath)
  } else {
    throw "envFilePath not valid"
  }

  const containerOptions = {
    Image: caaImage,
    Entrypoint: ["/bin/sh", "-c"],
    Cmd: [
      "echo nameserver 1.1.1.1 >> /etc/resolv.conf; /usr/local/bin/cloud-api-adaptor azure -disable-cvm -use-public-ip -subscriptionid $AZURE_SUBSCRIPTION_ID -region $AZURE_REGION -instance-size $AZURE_INSTANCE_SIZE -resourcegroup $AZURE_RESOURCE_GROUP -vxlan-port 8472 -subnetid $AZURE_SUBNET_ID -securitygroupid $AZURE_NSG_ID -imageid $AZURE_IMAGE_ID"
    ],
    EnvFiles: envFiles,
    Labels: { "peer-pods-service": "true" },
    Volumes: { "/run/peerpod:/run/peerpod:z": {} },
    Start: true,
    Detach: true
  }

  const connectionName = 'Podman Machine'
  const engine = extensionApi.provider.getContainerConnections()
    .filter(connection => connection.connection.type === 'podman')
    .find(connection => connection.connection.displayName === connectionName)
  if (!engine) {
    throw new Error(`no podman connection found with name ${connectionName}`);
  }

  const image = containerOptions.Image

  await extensionApi.containerEngine.pullImage(engine.connection, image, _event => { })

  const imageInfo = (await extensionApi.containerEngine.listImages({
    provider: engine.connection,
  } as extensionApi.ListImagesOptions)).find(imageInfo => imageInfo.RepoTags?.some(tag => tag === image))

  if (imageInfo === undefined) { throw new Error(`image ${image} not found.`) }

  await extensionApi.containerEngine.createContainer(imageInfo.engineId, containerOptions)

  extensionApi.context.setValue("peerpodsIsInstalled", true, "onboarding")
})

async function updateConfiguration() {
  // TODO
}

function watchConfiguration() {
  extensionApi.configuration.onDidChangeConfiguration(async e => {
    if (e.affectsConfiguration("peerpods")) {
      await updateConfiguration();
    }
  })
}

export async function activate(extensionContext: ExtensionContext): Promise<void> {
  extensionApi.context.setValue("peerpodsIsInstalled", false, "onboarding")

  extensionContext.subscriptions.push(
    setupMachine,
    startPeerpods
  )
}

export async function deactivate(): Promise<void> {
  extensionApi.context.setValue("peerpodsIsInstalled", false, "onboarding")
  extensionApi.window.showWarningMessage("To fully deactivate the extension, reset the Podman Machine by running the following command:\n\n    podman machine reset");
}
