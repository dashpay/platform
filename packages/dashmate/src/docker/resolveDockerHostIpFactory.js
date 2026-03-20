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

    const hostConfig = {};

    if (platform !== 'darwin' && platform !== 'win32' && !isWSL()) {
      hostConfig.ExtraHosts = ['host.docker.internal:host-gateway'];
    }

    const stdoutStream = new WritableStream();
    const stderrStream = new WritableStream();

    const [result] = await docker.run(
      'alpine',
      [],
      [stdoutStream, stderrStream],
      {
        Entrypoint: ['sh', '-c', 'ping -c1 host.docker.internal | sed -nE \'s/^PING[^(]+\\(([^)]+)\\).*/\\1/p\''],
        HostConfig: hostConfig,
      },
      {},
    ).catch(async (err) => {
      // docker.run with AutoRemove can race on container cleanup
      // If the error is a 404 "no such container", the run succeeded
      // but the container was already removed before wait() completed
      if (err.statusCode === 404) {
        // Container ran and was auto-removed; read what we captured
        const ip = stdoutStream.toString().trim();
        if (ip && /^\d+\.\d+\.\d+\.\d+$/.test(ip)) {
          return [{ StatusCode: 0 }];
        }
      }
      throw err;
    });

    const output = stdoutStream.toString();

    if (result.StatusCode !== 0) {
      throw new Error(`Can't get host.docker.internal IP address: ${output}`);
    }

    return output.trim();
  }

  return resolveDockerHostIp;
}
