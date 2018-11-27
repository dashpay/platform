/**
 * @param coreAPI
 * @return {getPeerDataSyncStatus}
 */
const getPeerDataSyncStatusFactory = (coreAPI) => {
  /**
   * Layer 1 endpoint
   * @typedef getPeerDataSyncStatus;
   * @return {Promise<object>}
   */
  function getPeerDataSyncStatus() {
    return coreAPI.getPeerDataSyncStatus();
  }

  return getPeerDataSyncStatus;
};

module.exports = getPeerDataSyncStatusFactory;
