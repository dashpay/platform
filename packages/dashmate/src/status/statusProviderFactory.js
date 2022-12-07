const scopes = require('./scopes');

/**
 *
 * @param {DockerCompose} dockerCompose
 * @param {createRpcClient} createRpcClient
 * @return {statusProvider}
 */
function statusProviderFactory(dockerCompose, createRpcClient) {
  return {
    getCoreScope: async (config) => scopes.core(createRpcClient, dockerCompose, config),
    getMasternodeScope: async (config) => scopes.masternode(createRpcClient, dockerCompose, config),
    getPlatformScope: async (config) => scopes.platform(createRpcClient, dockerCompose, config),
    getHostScope: async (config) => scopes.host(createRpcClient, dockerCompose, config),
    getServicesScope: async (config) => scopes.services(createRpcClient, dockerCompose, config),
    getOverviewScope: async (config) => scopes.overview(createRpcClient, dockerCompose, config),
  };
}

module.exports = statusProviderFactory;
