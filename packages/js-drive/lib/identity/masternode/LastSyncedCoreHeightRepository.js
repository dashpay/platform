const StorageResult = require('../../storage/StorageResult');

class LastSyncedCoreHeightRepository {
  /**
   *
   * @param {GroveDBStore} groveDBStore
   */
  constructor(groveDBStore) {
    this.storage = groveDBStore;
  }

  /**
   * Store last synced core height
   *
   * @param {number} height
   * @param {Object} [options]
   * @param {GroveDBTransaction} [options.transaction]
   * @param {boolean} [options.dryRun=false]
   * @return {Promise<StorageResult<void>>}
   */
  async store(height, options = {}) {
    const value = Buffer.alloc(4);

    value.writeUInt32BE(height);

    return this.storage.put(
      LastSyncedCoreHeightRepository.TREE_PATH,
      LastSyncedCoreHeightRepository.KEY,
      value,
      options,
    );
  }

  /**
   * Fetch last synced core height
   *
   * @param {Object} [options]
   * @param {GroveDBTransaction} [options.transaction]
   * @param {boolean} [options.dryRun=false]
   * @return {Promise<StorageResult<number|null>>}
   */
  async fetch(options = {}) {
    const encodedHeightResult = await this.storage.get(
      LastSyncedCoreHeightRepository.TREE_PATH,
      LastSyncedCoreHeightRepository.KEY,
      {
        ...options,
        predictedValueSize: 4,
      },
    );

    if (encodedHeightResult.isNull()) {
      return encodedHeightResult;
    }

    const encodedHeight = encodedHeightResult.getValue();

    return new StorageResult(
      encodedHeight.readUInt32BE(),
      encodedHeightResult.getOperations(),
    );
  }
}

LastSyncedCoreHeightRepository.TREE_PATH = [Buffer.from([5])];
LastSyncedCoreHeightRepository.KEY = Buffer.from('lastSyncedCoreHeight');

module.exports = LastSyncedCoreHeightRepository;
