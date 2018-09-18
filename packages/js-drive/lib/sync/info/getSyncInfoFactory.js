const SyncInfo = require('./SyncInfo');

/**
 * @param {SyncStateRepository} syncStateRepository
 * @param {getChainInfo} getChainInfo
 * @returns {getSyncInfo}
 */
function getSyncInfoFactory(syncStateRepository, getChainInfo) {
  /**
   * @typedef getSyncInfo
   * @returns {Promise<SyncInfo>}
   */
  async function getSyncInfo() {
    const syncState = await syncStateRepository.fetch();
    const lastDriveBlock = syncState.getLastBlock();
    const chainInfo = await getChainInfo();

    return new SyncInfo(
      lastDriveBlock.height,
      lastDriveBlock.hash,
      syncState.getLastSyncAt(),
      chainInfo.getLastBlockHeight(),
      chainInfo.getLastBlockHash(),
      chainInfo.getIsBlockchainSynced(),
    );
  }

  return getSyncInfo;
}

module.exports = getSyncInfoFactory;
