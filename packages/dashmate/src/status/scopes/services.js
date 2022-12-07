const ContainerIsNotPresentError = require('../../docker/errors/ContainerIsNotPresentError');

module.exports = async (createRpcClient, dockerCompose, config) => {
  const services = {};

  const serviceHumanNames = {
    core: 'Core',
  };

  if (config.get('core.masternode.enable')) {
    Object.assign(serviceHumanNames, {
      sentinel: 'Sentinel',
    });
  }

  if (config.get('network') !== 'mainnet') {
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
    } catch (e) {
      if (e instanceof ContainerIsNotPresentError) {
        status = 'not_started';
      }
    }

    services[serviceName] = {
      humanName: serviceDescription,
      containerId: containerId ? containerId.slice(0, 12) : null,
      image,
      status,
    };
  }

  return services;
};
