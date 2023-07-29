/**
 * @param {DockerCompose} dockerCompose
 * @param {boolean} isHelper
 * @return {getConnectionHost}
 */
function getConnectionHostFactory(dockerCompose, isHelper) {
  /**
   * Get proper service endpoint url
   * @typedef {function} getConnectionHost
   * @param {Config} config
   * @param {string} serviceName
   * @return {Promise<string>}
   */
  async function getConnectionHost(config, serviceName) {
    if (isHelper) {
      const containerInfo = await dockerCompose.inspectService(config, serviceName);

      const [firstNetwork] = Object.keys(containerInfo.NetworkSettings.Networks);
      const { IPAddress: containerIP } = containerInfo.NetworkSettings.Networks[firstNetwork];

      return containerIP;
    }

    return '127.0.0.1';
  }

  return getConnectionHost;
}

module.exports = getConnectionHostFactory;
