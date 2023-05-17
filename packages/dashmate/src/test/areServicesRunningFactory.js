/**
 * @param {Config[]} configGroup
 * @param {DockerCompose} dockerCompose
 * @param {Object} services
 *
 * @returns {areServicesRunning}
 */
function areServicesRunningFactory(configGroup, dockerCompose, services) {
  /**
   * Check all node services are up and running
   *
   * @returns {Promise<void>}
   */
  async function areServicesRunning() {
    for (const config of configGroup) {
      if (config.name === 'local_seed') {
        const result = await dockerCompose.isServiceRunning(config.toEnvs(), 'core');
        if (!result) {
          throw new Error('Core in local_seed is not running');
        }
      } else {
        for (const serviceName of Object.keys(services)) {
          const result = await dockerCompose.isServiceRunning(config.toEnvs(), serviceName);
          if (!result) {
            throw new Error(`Service ${serviceName} in ${config.name} is not running`);
          }
        }
      }
    }
  }

  return areServicesRunning;
}

module.exports = areServicesRunningFactory;
