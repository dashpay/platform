async function waitUntilBlockchainIsSynced(getSyncInfo, checkInterval) {
  return new Promise((resolve, reject) => {
    async function checkStatus() {
      const lastSyncInfo = await getSyncInfo();
      if (lastSyncInfo.getIsBlockchainSynced()) {
        return resolve(lastSyncInfo);
      }
      return setTimeout(checkStatus, checkInterval);
    }
    checkStatus().catch(reject);
  });
}

async function isDriveFullySynced(syncInfo) {
  if (syncInfo.getLastChainBlockHeight() === 0) {
    return true;
  }

  if (!syncInfo.getLastSyncedBlockHash()) {
    return false;
  }

  return syncInfo.getLastSyncedBlockHash() === syncInfo.getLastChainBlockHash();
}

async function waitUntilDriveIsSynced(stateRepositoryChangeListener, syncInfo) {
  return new Promise((resolve, reject) => {
    const changeHandler = (updatedSyncState) => {
      if (!updatedSyncState.getLastSyncAt()) {
        return;
      }

      if (syncInfo.getLastSyncAt()
          && (updatedSyncState.getLastSyncAt().getTime() === syncInfo.getLastSyncAt().getTime())) {
        return;
      }

      stateRepositoryChangeListener.removeListener('change', changeHandler);
      stateRepositoryChangeListener.stop();

      resolve(updatedSyncState);
    };

    stateRepositoryChangeListener.on('change', changeHandler);
    stateRepositoryChangeListener.on('error', reject);

    stateRepositoryChangeListener.listen();
  });
}

/**
 * Check is sync process complete
 *
 * @param {getSyncInfo} getSyncInfo
 * @param {SyncStateRepositoryChangeListener} stateRepositoryChangeListener
 * @param {number} checkInterval
 * @return {Promise<SyncState>}
 */
module.exports = async function isSynced(
  getSyncInfo,
  stateRepositoryChangeListener,
  checkInterval,
) {
  const lastSyncInfo = await waitUntilBlockchainIsSynced(getSyncInfo, checkInterval);

  const driveSynced = await isDriveFullySynced(lastSyncInfo);
  if (driveSynced) {
    return lastSyncInfo;
  }

  return waitUntilDriveIsSynced(stateRepositoryChangeListener, lastSyncInfo);
};
