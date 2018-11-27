/**
 * @param coreAPI
 * @return {getHistoricBlockchainDataSyncStatus}
 */
const getHistoricBlockchainDataSyncStatusFactory = (coreAPI) => {
  /**
   * Layer 1 endpoint
   * Returns sync status of the node
   * @typedef getHistoricBlockchainDataSyncStatus
   * @return {Promise<object>}
   */
  function getHistoricBlockchainDataSyncStatus() {
    return coreAPI.getHistoricBlockchainDataSyncStatus();
  }

  return getHistoricBlockchainDataSyncStatus;
};

module.exports = getHistoricBlockchainDataSyncStatusFactory;
