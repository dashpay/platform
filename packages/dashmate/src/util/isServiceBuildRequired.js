/**
 * Check if any of the service requires a build
 *
 * @param {Config} config
 *
 * @returns {boolean}
 */
export default function isServiceBuildRequired(config) {
  const isDashmateBuildRequired = config.get('dashmate.helper.docker.build.enabled');
  const isDriveBuildRequired = config.get('platform.enable') && config.get('platform.drive.abci.docker.build.enabled');
  const isDapiBuildRequired = config.get('platform.enable') && config.get('platform.dapi.rsDapi.docker.build.enabled');

  return isDashmateBuildRequired
    || isDriveBuildRequired
    || isDapiBuildRequired;
}
