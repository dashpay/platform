const STHeadersReader = require('../../blockchain/reader/STHeadersReader');

/**
 * Persist sync state
 *
 * @param {STHeadersReader} stHeadersReader
 * @param {SyncState} syncState
 * @param {SyncStateRepository} syncStateRepository
 */
function attachSyncHandlers(stHeadersReader, syncState, syncStateRepository) {
  const readerState = stHeadersReader.getState();

  async function saveState() {
    syncState.setBlocks(readerState.getBlocks());

    await syncStateRepository.store(syncState);
  }

  stHeadersReader.on(STHeadersReader.EVENTS.BLOCK, saveState);
  stHeadersReader.on(STHeadersReader.EVENTS.STALE_BLOCK, saveState);

  stHeadersReader.on(STHeadersReader.EVENTS.END, async () => {
    syncState.setBlocks(readerState.getBlocks());
    syncState.setLastSyncAt(new Date());

    await syncStateRepository.store(syncState);
  });
}

module.exports = attachSyncHandlers;
