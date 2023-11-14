/* eslint-disable camelcase */
import { promisify } from 'util';
import path from 'path';
import child_process from 'child_process';

const exec = promisify(child_process.exec);

/**
 * Returns docker socket path
 * @returns {Promise<string>}
 */
export default async function resolveDockerSocketPath() {
  const { stdout } = await exec('docker context inspect');

  const output = JSON.parse(stdout);

  return path.normalize(output[0].Endpoints.docker.Host.split(':').pop());
}
