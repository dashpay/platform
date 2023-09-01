const ContainerIsNotPresentError = require('../../docker/errors/ContainerIsNotPresentError');
const DockerStatusEnum = require('../enums/dockerStatus');

/**
 * @param {DockerCompose} dockerCompose
 * @param getServiceList
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

    for (const { name, title, image } of services) {
      let containerId;
      let status;

      try {
        ({
          Id: containerId,
          State: {
            Status: status,
          },
        } = await dockerCompose.inspectService(config, name));

        scope[name] = {
          title,
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

        scope[name] = {
          title,
          containerId: null,
          image,
          status,
        };
      }
    }

    return scope;
  }

  return getServicesScope;
}

module.exports = getServicesScopeFactory;
