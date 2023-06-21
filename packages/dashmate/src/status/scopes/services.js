const ContainerIsNotPresentError = require('../../docker/errors/ContainerIsNotPresentError');
const DockerStatusEnum = require('../enums/dockerStatus');
const generateEnvs = require('../../util/generateEnvs');

/**
 * @param {DockerCompose} dockerCompose
 * @param {ConfigFile} configFile
 * @returns {getServicesScopeFactory}
 */
function getServicesScopeFactory(dockerCompose, getServiceList) {
  /**
   * Get platform status scope
   *
   * @typedef {Promise} getPlatformScope
   * @param {Config} config
   * @returns {Promise<Object>}
   */
  async function getServicesScope(config) {
    const services = getServiceList(config);

    const scope = {};

    for (const { serviceName, humanName } of services) {
      let containerId;
      let status;
      let image;

      try {
        ({
          Id: containerId,
          State: {
            Status: status,
          },
          Config: {
            Image: image,
          },
        } = await dockerCompose.inspectService(generateEnvs(configFile, config), serviceName));

        scope[serviceName] = {
          humanName,
          containerId: containerId ? containerId.slice(0, 12) : null,
          image,
          status,
        };
      } catch (e) {
        status = null;

        if (e instanceof ContainerIsNotPresentError) {
          status = DockerStatusEnum.not_started;
        } else if (process.env.DEBUG) {
          // eslint-disable-next-line no-console
          console.error(e);
        }

        scope[serviceName] = {
          humanName,
          containerId: null,
          image: null,
          status,
        };
      }
    }

    return scope;
  }

  return getServicesScope;
}

module.exports = getServicesScopeFactory;
