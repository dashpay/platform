/**
 * @param {Config} config
 * @param {DockerCompose} dockerCompose
 *
 * @returns {isServicesRunning}
 */
function isServiceRunningFactory(config, dockerCompose) {
  /**
   * Check if service is running
   *
   * @param {string} serviceName
   *
   * @returns {Promise<boolean>}
   */
  async function isServicesRunning(serviceName) {
    return dockerCompose.isServiceRunning(
      config,
      serviceName,
    );
  }

  return isServicesRunning;
}

module.exports = isServiceRunningFactory;
