import type { ExtensionContext } from '@podman-desktop/api';
import * as extensionApi from '@podman-desktop/api';

const machineImage = "quay.io/spotlesstofu/podman-cri:5.1"

async function execPodman(args) {
  var command = "podman";
  return await extensionApi.process.exec(command, args)
}

export async function activate(extensionContext: ExtensionContext): Promise<void> {
  const setupMachine = extensionApi.commands.registerCommand('peerpods.onboarding.setupMachine', async () => {
    // machine init
    try {
      await execPodman(["machine", "init", "--now"]);
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

    extensionApi.context.setValue("peerpodsIsInstalled", "true", "onboarding")
  })

  const cloudConfig = extensionApi.commands.registerCommand('peerpods.onboarding.cloudConfig', async () => {
  })

  const startPeerpods = extensionApi.commands.registerCommand('peerpods.onboarding.startPeerpods', async () => {
  })

  extensionContext.subscriptions.push(
    setupMachine,
    cloudConfig,
    startPeerpods
  )

  extensionApi.context.setValue("peerpodsIsInstalled", "false", "onboarding")
}

export async function deactivate(): Promise<void> {
  extensionApi.window.showWarningMessage("To fully deactivate the extension, reset the Podman Machine by running the following command:\n\n    podman machine reset");
}
