function isServiceRunningFactory(config, dockerCompose, services) {
  async function isServicesRunning(serviceName) {
    return await dockerCompose.isServiceRunning(config.toEnvs(), serviceName)
  }

  return isServicesRunning
}

module.exports = isServiceRunningFactory;
