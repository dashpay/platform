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
) {
  return {
    getCoreScope: async (config) => scopes.core(dockerCompose, config),
    getMasternodeScope: async (config) => scopes.masternode(dockerCompose, config),
    getPlatformScope: async (config) => scopes.platform(dockerCompose, config),
    getHostScope: async (config) => scopes.host(dockerCompose, config),
    getServicesScope: async (config) => scopes.services(dockerCompose, config),
    getOverviewScope: async (config) => scopes.overview(dockerCompose, config),
  };
}

module.exports = statusProviderFactory;
