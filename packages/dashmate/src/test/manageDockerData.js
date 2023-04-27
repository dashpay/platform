const { getConfig } = require('./manageConfig');
const { SERVICES } = require('./constants/services');

/**
 * Remove docker volumes
 * @param {string} configName
 * @param {object} dockerContainer
 */
async function removeVolumes(configName, dockerContainer) {
  const config = await getConfig(configName);
  const { COMPOSE_PROJECT_NAME: projectName } = config.toEnvs();
  const projectVolumeNames = await dockerContainer.getVolumeNames(config.toEnvs());

  await Promise.all(
    projectVolumeNames
      .map((volumeName) => `${projectName}_${volumeName}`)
      .map(async (volumeName) => dockerContainer.getVolume(volumeName).remove()),
  );
}

/**
 * Remove docker containers
 * @param {string} configName
 * @param {object} dockerContainer
 */
async function removeContainers(configName, dockerContainer) {
  const options = ['--services', '--status=running'];

  const config = await getConfig(configName);
  const getContainers = await dockerContainer.getContainersList(config.toEnvs(), options, true);
  // await dockerContainer.stop(config.toEnvs(), getContainers);
  await dockerContainer.rm(config.toEnvs(), getContainers);
}

/**
 * Check if containers running for group of local nodes
 * @param {boolean} isRunning
 * @param {object} dockerContainer
 */
async function isGroupServicesRunning(isRunning, dockerContainer) {
  let result;

  const [groupConfig] = await Promise.all([getConfig('local')]);

  for (const config of groupConfig) {
    if (config.name === 'local_seed') {
      result = await dockerContainer.isServiceRunning(config.toEnvs(), 'core');
      if (result !== isRunning) {
        throw new Error(`Running state for core in local_seed is not ${isRunning}`);
      }
    } else {
      for (const [key] of Object.entries(SERVICES)) {
        result = await dockerContainer.isServiceRunning(config.toEnvs(), key);
        if (result !== isRunning) {
          throw new Error(`Running state for ${key} in ${config.name} is not ${isRunning}`);
        }
      }
    }
  }
}

/**
 * Check if containers running for testnet node
 * @param {boolean} isRunning
 * @param {object} dockerContainer
 */
async function isTestnetServicesRunning(isRunning, dockerContainer) {
  const config = await Promise.all([getConfig('testnet')]);

  for (const [key] of Object.entries(SERVICES)) {
    const result = await dockerContainer.isServiceRunning(config.toEnvs(), key);
    if (result !== isRunning) {
      throw new Error(`Running state for service ${key} is not ${isRunning}`);
    }
  }
}

module.exports = {
  removeVolumes,
  removeContainers,
  isGroupServicesRunning,
  isTestnetServicesRunning,
};
