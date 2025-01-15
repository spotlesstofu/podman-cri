import type { ExtensionContext } from '@podman-desktop/api';
import * as extensionApi from '@podman-desktop/api';

/**
 * Below is the "typical" extension.ts file that is used to activate and deactivate the extension.
 * this file as well as package.json are the two main files that are required to develop a Podman Desktop extension.
 */

// Initialize the activation of the extension.
export async function activate(extensionContext: ExtensionContext): Promise<void> {
  var tag = extensionApi.version;
  var image = `quay.io/spotlesstofu/podman-cri:${tag}`;
  var command = "podman"; 
  var args = ["machine", "os", "apply", "--restart", image];
  try {
    await extensionApi.process.exec(command, args);
  } catch (e) {
    console.error(e)
  }
}

// Deactivate the extension
export async function deactivate(): Promise<void> {
  console.log('stopping hello world extension');
}
