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
   * @param {boolean} [options.useTransaction=false]
   * @param {boolean} [options.dryRun=false]
   *
   * @return {Promise<StorageResult<void>>}
   */
  async store(outPointBuffer, options = {}) {
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
   * @param {boolean} [options.useTransaction=false]
   * @param {boolean} [options.dryTun=false]
   *
   * @return {Promise<StorageResult<null|Buffer>>}
   */
  async fetch(outPointBuffer, options = {}) {
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

  /**
   * @param {Object} [options]
   * @param {boolean} [options.useTransaction=false]
   * @param {boolean} [options.skipIfExists=false]
   * @param {boolean} [options.dryRun=false]
   *
   * @return {Promise<StorageResult<void>>}
   */
  async createTree(options = {}) {
    const rootTreePath = [SpentAssetLockTransactionsRepository.TREE_PATH[0]];
    const treePath = SpentAssetLockTransactionsRepository.TREE_PATH[1];

    const result = await this.storage.createTree(
      rootTreePath,
      treePath,
      options,
    );

    return new StorageResult(
      undefined,
      result.getOperations(),
    );
  }
}

SpentAssetLockTransactionsRepository.TREE_PATH = [
  Buffer.from([3]),
];

module.exports = SpentAssetLockTransactionsRepository;
