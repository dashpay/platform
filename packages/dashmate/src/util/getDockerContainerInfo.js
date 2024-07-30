/**
 * A wrapper function to get container status, exit code, and logs
 * @param config {Config}
 * @param service {string}
 * @param dockerCompose {DockerCompose}
 * @returns {Promise<{stdOut: *, exitCode: *, stdErr: *, status}>}
 */
export default async function getDockerContainerInfo(config, service, dockerCompose) {
  const info = await dockerCompose.inspectService(config, service);

  const { exitCode, err: stdErr, out: stdOut } = await dockerCompose.logs(config, [service]);

  return {
    status: info.State.Status, exitCode, stdOut, stdErr,
  };
}
