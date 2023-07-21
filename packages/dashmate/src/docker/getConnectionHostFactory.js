const generateEnvs = require('../util/generateEnvs');

function getConnectionHostFactory(dockerCompose, isHelper, configFile) {
  /**
   * Get proper service endpoint url
   * @param config
   * @param serviceName
   * @return {Promise<string>}
   */
  async function getConnectionHost(config, serviceName) {
    if (isHelper) {
      const envs = generateEnvs(configFile, config);
      const containerInfo = await dockerCompose.inspectService(envs, serviceName);

      const [firstNetwork] = Object.keys(containerInfo.NetworkSettings.Networks);
      const { IPAddress: containerIP } = containerInfo.NetworkSettings.Networks[firstNetwork];

      return containerIP;
    }

    return '127.0.0.1';
  }

  return getConnectionHost;
}

module.exports = getConnectionHostFactory;
