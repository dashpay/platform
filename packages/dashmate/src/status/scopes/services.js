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

    for (const { name, title } of services) {
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
