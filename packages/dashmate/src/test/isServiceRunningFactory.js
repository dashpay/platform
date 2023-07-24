const generateEnvs = require('../config/generateEnvsFactory');

/**
 * @param {Config} config
 * @param {ConfigFile} configFile
 * @param {DockerCompose} dockerCompose
 *
 * @returns {isServicesRunning}
 */
function isServiceRunningFactory(config, configFile, dockerCompose) {
  /**
   * Check if service is running
   *
   * @param {string} serviceName
   *
   * @returns {Promise<boolean>}
   */
  async function isServicesRunning(serviceName) {
    return dockerCompose.isServiceRunning(
      generateEnvs(configFile, config),
      serviceName,
    );
  }

  return isServicesRunning;
}

module.exports = isServiceRunningFactory;
