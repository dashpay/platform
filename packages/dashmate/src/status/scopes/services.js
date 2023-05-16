const ContainerIsNotPresentError = require('../../docker/errors/ContainerIsNotPresentError');
const DockerStatusEnum = require('../enums/dockerStatus');

/**
 * @returns {getServicesScopeFactory}
 * @param dockerCompose {DockerCompose}
 */
function getServicesScopeFactory(dockerCompose) {
  /**
   * Get platform status scope
   *
   * @typedef {Promise} getPlatformScope
   * @param {Config} config
   * @returns {Promise<Object>}
   */
  async function getServicesScope(config) {
    const services = {};

    const serviceHumanNames = {
      core: 'Core',
    };

    if (config.get('core.masternode.enable')) {
      Object.assign(serviceHumanNames, {
        sentinel: 'Sentinel',
      });
    }

    if (config.get('platform.enable')) {
      Object.assign(serviceHumanNames, {
        drive_abci: 'Drive ABCI',
        drive_tenderdash: 'Drive Tenderdash',
        dapi_api: 'DAPI API',
        dapi_tx_filter_stream: 'DAPI Transactions Filter Stream',
        dapi_envoy: 'DAPI Envoy',
      });
    }

    for (const [serviceName, serviceDescription] of Object.entries(serviceHumanNames)) {
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
        } = await dockerCompose.inspectService(config.toEnvs(), serviceName));

        services[serviceName] = {
          humanName: serviceDescription,
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

        services[serviceName] = {
          humanName: serviceDescription,
          containerId: null,
          image: null,
          status,
        };
      }
    }

    return services;
  }

  return getServicesScope;
}

module.exports = getServicesScopeFactory;
