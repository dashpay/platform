/**
 * @return {getServiceList}
 */
function getServiceListFactory() {
  /**
   * Returns list of services and corresponding docker images from the config
   * @typedef {getServiceList}
   * @return {Config} config
   */
  function getServiceList(config) {
    const services = {
      core: {
        humanName: 'Core',
        image: config.get('core.docker.image'),
      },
    };

    if (config.get('core.masternode.enable')) {
      Object.assign(services, {
        sentinel: {
          humanName: 'Sentinel',
          image: config.get('core.sentinel.docker.image'),
        },
      });
    }

    if (config.get('platform.enable')) {
      Object.assign(services, {
        drive_abci: {
          humanName: 'Drive ABCI',
          image: config.get('platform.drive.abci.docker.image'),
        },
        drive_tenderdash: {
          humanName: 'Drive Tenderdash',
          image: config.get('platform.drive.tenderdash.docker.image'),
        },
        dapi_api: {
          humanName: 'DAPI API',
          image: config.get('platform.dapi.api.docker.image'),
        },
        dapi_tx_filter_stream: {
          humanName: 'DAPI Transactions Filter Stream',
          image: config.get('platform.drive.abci.docker.image'),
        },
        dapi_envoy: {
          humanName: 'DAPI Envoy',
          image: config.get('platform.dapi.envoy.docker.image'),
        },
        dashmate_helper: {
          humanName: 'Dashmate Helper',
          image: config.get('dashmate.helper.docker.image'),
        },
      });
    }

    return Object.keys(services).map((serviceName) => ({
      serviceName,
      humanName: services[serviceName].humanName,
      image: services[serviceName].image,
    }));
  }

  return getServiceList;
}

module.exports = getServiceListFactory;
