/**
 * @param {DockerCompose} dockerCompose
 * @param {boolean} isHelper
 * @return {getConnectionHost}
 */
export default function getConnectionHostFactory(dockerCompose, isHelper) {
  /**
   * Get proper service endpoint url
   * @typedef {function} getConnectionHost
   * @param {Config} config
   * @param {string} serviceName
   * @param {string} hostConfigurationPath
   * @return {Promise<string>}
   */
  async function getConnectionHost(config, serviceName, hostConfigurationPath) {
    if (isHelper) {
      const containerInfo = await dockerCompose.inspectService(config, serviceName);

      const [firstNetwork] = Object.keys(containerInfo.NetworkSettings.Networks);
      const { IPAddress: containerIP } = containerInfo.NetworkSettings.Networks[firstNetwork];

      return containerIP;
    }

    return config.get(hostConfigurationPath);
  }

  return getConnectionHost;
}
