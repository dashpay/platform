/**
 * @param coreAPI
 * @return {getPeerDataSyncStatus}
 */
const getPeerDataSyncStatusFactory = (coreAPI) => {
  /**
   * @typedef getPeerDataSyncStatus;
   * @return {Promise<object>}
   */
  function getPeerDataSyncStatus() {
    return coreAPI.getPeerDataSyncStatus();
  }

  return getPeerDataSyncStatus;
};

module.exports = getPeerDataSyncStatusFactory;
