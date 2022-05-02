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
   * @param {boolean} [useTransaction=false]
   *
   * @return {Promise<StorageResult<void>>}
   */
  async store(outPointBuffer, useTransaction = false) {
    const emptyValue = Buffer.from([0]);

    const result = await this.storage.put(
      SpentAssetLockTransactionsRepository.TREE_PATH,
      outPointBuffer,
      emptyValue,
      { useTransaction },
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
   * @param {boolean} [useTransaction=false]
   *
   * @return {Promise<StorageResult<null|Buffer>>}
   */
  async fetch(outPointBuffer, useTransaction = false) {
    const result = await this.storage.get(
      SpentAssetLockTransactionsRepository.TREE_PATH,
      outPointBuffer,
      { useTransaction },
    );

    return new StorageResult(
      result.getValue(),
      result.getOperations(),
    );
  }

  /**
   * @param {Object} [options]
   * @param {boolean} [options.useTransaction=false]
   * @param {boolean} [options.skipIfExists]
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
  Buffer.from([0]),
];

module.exports = SpentAssetLockTransactionsRepository;
