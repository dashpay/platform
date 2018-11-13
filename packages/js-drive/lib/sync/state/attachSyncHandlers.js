const ReaderMediator = require('../../blockchain/reader/BlockchainReaderMediator');

/**
 * Persist sync state
 *
 * @param {BlockchainReaderMediator} readerMediator
 * @param {SyncState} syncState
 * @param {SyncStateRepository} syncStateRepository
 */
function attachSyncHandlers(readerMediator, syncState, syncStateRepository) {
  async function saveState() {
    syncState.setBlocks(readerMediator.getState().getBlocks());

    await syncStateRepository.store(syncState);
  }

  let isInitialSyncFinished = false;

  readerMediator.on(ReaderMediator.EVENTS.FULLY_SYNCED, () => {
    isInitialSyncFinished = true;
  });

  readerMediator.on(ReaderMediator.EVENTS.RESET, () => {
    isInitialSyncFinished = false;
  });

  readerMediator.on(ReaderMediator.EVENTS.BLOCK_END, saveState);
  readerMediator.on(ReaderMediator.EVENTS.BLOCK_STALE, saveState);

  readerMediator.on(ReaderMediator.EVENTS.END, async () => {
    syncState.setLastSyncAt(new Date());

    if (!isInitialSyncFinished) {
      syncState.setLastInitialSyncAt(new Date());
      isInitialSyncFinished = true;
    }

    await syncStateRepository.store(syncState);
  });
}

module.exports = attachSyncHandlers;
