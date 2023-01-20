async function getConnectionHost(dockerCompose, config, serviceName) {
  if (process.env.DASHMATE_HELPER === 'true') {
    return dockerCompose.getContainerIp(config.toEnvs(), serviceName);
  }

  return '127.0.0.1';
}

module.exports = getConnectionHost;
