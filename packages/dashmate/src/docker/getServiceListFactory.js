const { DASHMATE_HELPER_DOCKER_IMAGE } = require('../constants');

/**
 * @return {getServiceList}
 */
function getServiceListFactory() {
  /**
   * Returns list of services and corresponding docker images from the config
   *
   * @typedef {getServiceList}
   * @param {Config} config
   * @return {Object[]}
   */
  function getServiceList(config) {
    const services = [
      {
        name: 'dashmate_helper',
        title: 'Dashmate Helper',
        image: DASHMATE_HELPER_DOCKER_IMAGE,
      },
      {
        name: 'core',
        title: 'Core',
        image: config.get('core.docker.image'),
      },
    ];

    if (config.get('core.masternode.enable')) {
      services.push({
        name: 'sentinel',
        title: 'Sentinel',
        image: config.get('core.sentinel.docker.image'),
      });
    }

    if (config.get('platform.enable')) {
      services.push({
        name: 'drive_abci',
        title: 'Drive ABCI',
        image: config.get('platform.drive.abci.docker.image'),
      }, {
        name: 'drive_tenderdash',
        title: 'Drive Tenderdash',
        image: config.get('platform.drive.tenderdash.docker.image'),
      }, {
        name: 'dapi_api',
        title: 'DAPI API',
        image: config.get('platform.dapi.api.docker.image'),
      }, {
        name: 'dapi_tx_filter_stream',
        title: 'DAPI Transactions Filter Stream',
        image: config.get('platform.drive.abci.docker.image'),
      }, {
        name: 'dapi_envoy',
        title: 'DAPI Envoy',
        image: config.get('platform.dapi.envoy.docker.image'),
      });
    }

    return services;
  }

  return getServiceList;
}

module.exports = getServiceListFactory;
