/**
 * Persist sync state
 *
 * @param {STHeadersReader} stHeadersReader
 * @param {SyncState} syncState
 * @param {SyncStateRepository} syncStateRepository
 */
function attachSyncHandlers(stHeadersReader, syncState, syncStateRepository) {
  const readerState = stHeadersReader.getState();
  stHeadersReader.on('block', async () => {
    syncState.setBlocks(readerState.getBlocks());

    await syncStateRepository.store(syncState);
  });
  stHeadersReader.on('end', async () => {
    syncState.setLastSyncAt(new Date());

    await syncStateRepository.store(syncState);
  });
}

module.exports = attachSyncHandlers;
