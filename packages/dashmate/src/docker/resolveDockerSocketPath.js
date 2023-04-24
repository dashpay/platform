const { promisify } = require('util');
const path = require('path');
const exec = promisify(require('child_process').exec);

/**
 * Returns docker socket path
 * @returns {Promise<string|undefined>}
 */
async function resolveDockerSocketPath() {
  try {
    const { stdout } = await exec('docker context inspect');

    const output = JSON.parse(stdout);

    return path.normalize(output[0].Endpoints.docker.Host.split(':').pop());
  } catch (e) {
    return undefined;
  }
}

module.exports = resolveDockerSocketPath;
