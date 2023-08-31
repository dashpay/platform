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
    const services = [
      {
        name: 'core',
        title: 'Core',
        image: config.get('core.docker.image'),
      },
    ];

    if (config.get('platform.enable')) {
      services.push({
        name: 'drive_abci',
        title: 'Drive ABCI',
        image: config.get('platform.drive.abci.docker.image'),
      },
      {
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
      }, {
        name: 'dashmate_helper',
        title: 'Dashmate Helper',
        image: config.get('dashmate.helper.docker.image'),
      });
    }

    return services;
  }

  return getServiceList;
}

module.exports = getServiceListFactory;
