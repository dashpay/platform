const generateEnvs = require('../config/generateEnvsFactory');

/**
 * @param {ConfigFile} configFile
 * @param {Config[]} configGroup
 * @param {DockerCompose} dockerCompose
 * @param {Object} services
 *
 * @returns {areServicesRunning}
 */
function areServicesRunningFactory(configFile, configGroup, dockerCompose, services) {
  /**
   * Check all node services are up and running
   *
   * @returns {Promise<boolean>}
   */
  async function areServicesRunning() {
    let result = true;

    for (const config of configGroup) {
      if (config.name === 'local_seed') {
        result = result && (await dockerCompose.isServiceRunning(
          generateEnvs(configFile, config),
          'core',
        ));
      } else {
        for (const serviceName of Object.keys(services)) {
          result = result && (await dockerCompose.isServiceRunning(
            generateEnvs(configFile, config),
            serviceName,
          ));
        }
      }
    }

    return result;
  }

  return areServicesRunning;
}

module.exports = areServicesRunningFactory;
