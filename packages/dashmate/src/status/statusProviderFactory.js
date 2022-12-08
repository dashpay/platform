const getCoreScopeFactory = require('./scopes/core');
const getMasternodeScopeFactory = require('./scopes/masternode');
const getPlatformScopeFactory = require('./scopes/platform');
const getHostScopeFactory = require('./scopes/host');
const getServicesScopeFactory = require('./scopes/services');
const getOverviewScopeFactory = require('./scopes/overview');
/**
 *
 * @param {DockerCompose} dockerCompose
 * @param {createRpcClient} createRpcClient
 * @return {statusProvider}
 */
function statusProviderFactory(dockerCompose, createRpcClient) {
  const getMasternodeScope = getMasternodeScopeFactory(dockerCompose, createRpcClient);
  const getPlatformScope = getPlatformScopeFactory(dockerCompose, createRpcClient);
  const getOverviewScope = getOverviewScopeFactory(dockerCompose, createRpcClient);
  const getServicesScope = getServicesScopeFactory(dockerCompose, createRpcClient);
  const getCoreScope = getCoreScopeFactory(dockerCompose, createRpcClient);
  const getHostScope = getHostScopeFactory();

  return {
    getCoreScope: async (config) => getCoreScope(config),
    getMasternodeScope: async (config) => getMasternodeScope(config),
    getPlatformScope: async (config) => getPlatformScope(config),
    getHostScope: async () => getHostScope(),
    getServicesScope: async (config) => getServicesScope(config),
    getOverviewScope: async (config) => getOverviewScope(config,
      getPlatformScope, getMasternodeScope),
  };
}

module.exports = statusProviderFactory;
