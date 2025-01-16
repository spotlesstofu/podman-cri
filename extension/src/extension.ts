import type { ExtensionContext } from '@podman-desktop/api';
import * as extensionApi from '@podman-desktop/api';

const machineImage = "quay.io/spotlesstofu/podman-cri:5.1"

async function execPodman(args) {
  var command = "podman";
  return await extensionApi.process.exec(command, args)
}

// Initialize the activation of the extension.
export async function activate(extensionContext: ExtensionContext): Promise<void> {
  // machine init
  try {
    await execPodman(["machine", "init", "--now"]);
  } catch (e) {
    if (e.stderr.includes("already exists")) {
        // pass, no need to create the machine
    } else {
      console.error(e.stderr);
      extensionApi.window.showWarningMessage(`Failed machine init: ${e.stderr}`);
      throw e;
    }
  }

  // TODO await machine is up

  // machine os apply
  try {
    await execPodman(["machine", "os", "apply", "--restart", machineImage]);
  } catch (e) {
    console.error(e.stderr);
    extensionApi.window.showWarningMessage(`Failed machine os apply: ${e.stderr}`);
    throw e;
  }
}

// Deactivate the extension
export async function deactivate(): Promise<void> {
  extensionApi.window.showWarningMessage("To fully deactivate the extension, reset the Podman Machine by running the following command:\n\n    podman machine reset");
}
