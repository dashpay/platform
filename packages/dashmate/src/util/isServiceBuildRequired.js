/**
 * Check if any of the service requires a build
 *
 * @param {Config} config
 *
 * @returns {boolean}
 */
function isServiceBuildRequired(config) {
  return (config.get('dashmate.helper.buildFromSource') === true)
    || (config.get('platform.drive.abci.buildFromSource') === true)
    || (config.get('platform.dapi.api.buildFromSource') === true)
    || (config.get('platform.dapi.txFilterStream.buildFromSource') === true)
    || (config.get('platform.dapi.envoy.buildFromSource') === true);
}

module.exports = isServiceBuildRequired;
