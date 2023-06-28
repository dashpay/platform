/**
 * Check if any of the service requires a build
 *
 * @param {Config} config
 *
 * @returns {boolean}
 */
function isServiceBuildRequired(config) {
  const isDashmateBuildRequired = config.get('dashmate.helper.dockerBuild.context') !== null
    || config.get('dashmate.helper.dockerBuild.dockerFilePath') !== null;

  const isDriveBuildRequired = config.get('platform.drive.abci.dockerBuild.context') !== null
    || config.get('platform.drive.abci.dockerBuild.dockerFilePath') !== null;

  const isDapiBuildRequired = config.get('platform.dapi.api.dockerBuild.context') !== null
    || config.get('platform.dapi.api.dockerBuild.dockerFilePath') !== null;

  const isEnvoyBuildRequired = config.get('platform.dapi.envoy.dockerBuild.context') !== null
    || config.get('platform.dapi.envoy.dockerBuild.dockerFilePath') !== null;

  return isDashmateBuildRequired
    || isDriveBuildRequired
    || isDapiBuildRequired
    || isEnvoyBuildRequired;
}

module.exports = isServiceBuildRequired;
