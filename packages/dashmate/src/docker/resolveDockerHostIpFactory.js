import os from 'os';
import { WritableStream } from 'memory-streams';
import isWSL from '../util/isWSL.js';

/**
 * @param {Docker} docker
 * @param {dockerPull} dockerPull
 * @return {resolveDockerHostIp}
 */
export default function resolveDockerHostIpFactory(docker, dockerPull) {
  /**
   * @typedef {resolveDockerHostIp}
   * @return {Promise<string>}
   */
  async function resolveDockerHostIp() {
    await dockerPull('alpine');

    const platform = os.platform();

    const hostConfig = {
      AutoRemove: true,
    };

    if (platform !== 'darwin' && platform !== 'win32' && !isWSL()) {
      hostConfig.ExtraHosts = ['host.docker.internal:host-gateway'];
    }

    const writableStream = new WritableStream();

    const [result] = await docker.run(
      'alpine',
      [],
      writableStream,
      {
        Entrypoint: ['sh', '-c', 'ping -c1 host.docker.internal | sed -nE \'s/^PING[^(]+\\(([^)]+)\\).*/\\1/p\''],
        HostConfig: hostConfig,
      },
      {},
    );

    const output = writableStream.toString();

    if (result.StatusCode !== 0) {
      throw new Error(`Can't get host.docker.internal IP address: ${output}`);
    }

    return output.trim();
  }

  return resolveDockerHostIp;
}
