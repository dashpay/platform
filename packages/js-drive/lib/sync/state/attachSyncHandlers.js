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

  readerMediator.on(ReaderMediator.EVENTS.BLOCK_END, saveState);
  readerMediator.on(ReaderMediator.EVENTS.BLOCK_STALE, saveState);

  readerMediator.on(ReaderMediator.EVENTS.END, async () => {
    syncState.updateLastSyncAt(new Date());

    await syncStateRepository.store(syncState);
  });
}

module.exports = attachSyncHandlers;
