const CoreService = require('../core/CoreService');

const scopes = require('./scopes');

/**
 *
 * @param {DockerCompose} dockerCompose
 * @param {createRpcClient} createRpcClient
 * @return {statusProvider}
 */
function statusProviderFactory(
  dockerCompose,
  createRpcClient,
  config,
) {
  const coreService = new CoreService(
    config,
    createRpcClient(
      {
        port: config.get('core.rpc.port'),
        user: config.get('core.rpc.user'),
        pass: config.get('core.rpc.password'),
      },
    ),
    dockerCompose.docker.getContainer('core'),
  );

  return {
    getCoreScope: async () => scopes.core(coreService, dockerCompose, config),
    getMasternodeScope: async () => scopes.masternode(coreService, dockerCompose, config),
    getPlatformScope: async () => scopes.platform(coreService, dockerCompose, config),
    getHostScope: async () => scopes.host(coreService, dockerCompose, config),
    getServicesScope: async () => scopes.services(coreService, dockerCompose, config),
    getOverviewScope: async () => scopes.overview(coreService, dockerCompose, config),
  };

  return statusProviderFactory;
}

module.exports = statusProviderFactory;
