const ContainerIsNotPresentError = require("../docker/errors/ContainerIsNotPresentError");

/**
 * Determine status based on the docker compose output
 * @param dockerCompose {DockerCompose}
 * @param config {Config}
 * @param serviceName {string}
 */
const determineStatus = async (dockerCompose, config, serviceName) => {
  try {
    const containerInfo = await dockerCompose.inspectService(config.toEnvs(), serviceName);

    return containerInfo.State.Status
  } catch (e) {
    if (e instanceof ContainerIsNotPresentError) {
      return 'not_started'
    }

    throw e
  }
}


module.exports = determineStatus
