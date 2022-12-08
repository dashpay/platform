const getCoreScopeFactory = require('../../src/status/scopes/core')
const getMasternodeScopeFactory = require('../../src/status/scopes/masternode')
const getPlatformScopeFactory = require('../../src/status/scopes/platform')
const getHostScopeFactory = require('../../src/status/scopes/host')
const getServicesScopeFactory = require('../../src/status/scopes/services')
const getOverviewScopeFactory = require('../../src/status/scopes/overview')
/**
 *
 * @param {DockerCompose} dockerCompose
 * @param {createRpcClient} createRpcClient
 * @return {statusProvider}
 */
function statusProviderFactory(dockerCompose, createRpcClient) {
  const getMasternodeScope = getMasternodeScopeFactory(dockerCompose, createRpcClient)
  const getPlatformScope = getPlatformScopeFactory(dockerCompose, createRpcClient)
  const getOverviewScope = getOverviewScopeFactory(dockerCompose, createRpcClient)
  const getServicesScope = getServicesScopeFactory(dockerCompose, createRpcClient)
  const getCoreScope = getCoreScopeFactory(dockerCompose, createRpcClient)
  const getHostScope = getHostScopeFactory()

  return {
    getCoreScope: async (config) => getCoreScope(config),
    getMasternodeScope: async (config) => getMasternodeScope(config),
    getPlatformScope: async (config) => getPlatformScope(config),
    getHostScope: async () => getHostScope(),
    getServicesScope: async (config) => getServicesScope(config),
    getOverviewScope: async (config) => getOverviewScope(config, getPlatformScope, getMasternodeScope),
  };
}

module.exports = statusProviderFactory;
