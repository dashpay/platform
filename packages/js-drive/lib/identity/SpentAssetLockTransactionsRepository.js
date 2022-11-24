const StorageResult = require('../storage/StorageResult');

class SpentAssetLockTransactionsRepository {
  /**
   * @param {GroveDBStore} groveDBStore
   */
  constructor(groveDBStore) {
    this.storage = groveDBStore;
  }

  /**
   * Store the outPoint
   *
   * @param {Buffer} outPointBuffer
   * @param {Object} [options]
   * @param {GroveDBTransaction} [options.transaction]
   * @param {boolean} [options.dryRun=false]
   *
   * @return {Promise<StorageResult<void>>}
   */
  async store(outPointBuffer, options = {}) {
    if (options.dryRun) {
      return new StorageResult(undefined, []);
    }

    const emptyValue = Buffer.from([0]);

    const result = await this.storage.put(
      SpentAssetLockTransactionsRepository.TREE_PATH,
      outPointBuffer,
      emptyValue,
      options,
    );

    return new StorageResult(
      undefined,
      result.getOperations(),
    );
  }

  /**
   * Fetch the outPoint
   *
   * @param {Buffer} outPointBuffer
   * @param {Object} [options]
   * @param {GroveDBTransaction} [options.transaction]
   * @param {boolean} [options.dryRun=false]
   *
   * @return {Promise<StorageResult<null|Buffer>>}
   */
  async fetch(outPointBuffer, options = {}) {
    if (options.dryRun) {
      return new StorageResult(null, []);
    }

    const result = await this.storage.get(
      SpentAssetLockTransactionsRepository.TREE_PATH,
      outPointBuffer,
      options,
    );

    return new StorageResult(
      result.getValue(),
      result.getOperations(),
    );
  }
}

SpentAssetLockTransactionsRepository.TREE_PATH = [
  Buffer.from([3]),
];

module.exports = SpentAssetLockTransactionsRepository;
