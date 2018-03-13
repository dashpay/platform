/**
 * Persist sync state
 *
 * @param {STHeadersReader} stHeadersReader
 * @param {SyncStateRepository} syncStateRepository
 */
module.exports = function attachStoreSyncStateHandler(stHeadersReader, syncStateRepository) {
  const syncState = stHeadersReader.getState();
  stHeadersReader.on('block', () => {
    syncStateRepository.store(syncState);
  });
};
