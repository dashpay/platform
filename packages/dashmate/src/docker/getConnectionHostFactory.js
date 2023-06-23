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
      return dockerCompose.getContainerIp(generateEnvs(configFile, config), serviceName);
    }

    return '127.0.0.1';
  }

  return getConnectionHost;
}

module.exports = getConnectionHostFactory;
