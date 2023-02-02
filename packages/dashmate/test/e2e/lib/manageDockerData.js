const { getConfig } = require("./manageConfig");
const { SERVICES } = require("./constants/services");

async function removeVolumes(configName, dockerContainer) {
  const config = await getConfig(configName)
  const {COMPOSE_PROJECT_NAME: projectName} = config.toEnvs();
  const projectVolumeNames = await dockerContainer.getVolumeNames(config.toEnvs());

  projectVolumeNames
    .map((volumeName) => `${projectName}_${volumeName}`)
    .map(async (volumeName) => await dockerContainer.getVolume(volumeName).remove());
}

async function removeContainers(configName, dockerContainer) {
  const commandOptions = ['--services', '--status=running'];

  const config = await getConfig(configName)
  const getContainers = await dockerContainer.getContainersList(config.toEnvs(), commandOptions, true);
  // await dockerContainer.stop(config.toEnvs(), getContainers);
  await dockerContainer.rm(config.toEnvs(), getContainers);
}

async function isGroupServicesRunning(isRunning, dockerContainer) {
  let result;
  const configFile = await getConfig('local')
  for (const config of configFile) {
    for (const [key] of Object.entries(SERVICES)) {
      if (config.name === 'local_seed') {
        result = await dockerContainer.isServiceRunning(config.toEnvs(), SERVICES.core);
      } else {
        result = await dockerContainer.isServiceRunning(config.toEnvs(), SERVICES[key]);
      }

      if (result !== isRunning) {
        throw new Error(`Running state for service ${key} should be ${isRunning}`)
      }
    }
  }
}

async function isTestnetServicesRunning(isRunning, dockerContainer) {
  const configFile = await getConfig('testnet')
  for (const [key] of Object.entries(SERVICES)) {
    const result = await dockerContainer.isServiceRunning(configFile.toEnvs(), SERVICES[key]);
    console.log(`${SERVICES[key]}: ${result}`)
    if (result !== isRunning) {
      throw new Error(`Running state for service ${key} should be ${isRunning}`)
    }
  }
}

module.exports = {
  removeVolumes,
  removeContainers,
  isGroupServicesRunning,
  isTestnetServicesRunning
}
