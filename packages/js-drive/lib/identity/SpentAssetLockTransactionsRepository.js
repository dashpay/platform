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
   * @return {SpentAssetLockTransactionsRepository}
   */
  async store(outPointBuffer, useTransaction = false) {
    const emptyValue = Buffer.from([0]);

    await this.storage.put(
      SpentAssetLockTransactionsRepository.TREE_PATH,
      outPointBuffer,
      emptyValue,
      { useTransaction },
    );

    return this;
  }

  /**
   * Fetch the outPoint
   *
   * @param {Buffer} outPointBuffer
   * @param {boolean} [useTransaction=false]
   *
   * @return {null|Buffer}
   */
  async fetch(outPointBuffer, useTransaction = false) {
    return this.storage.get(
      SpentAssetLockTransactionsRepository.TREE_PATH,
      outPointBuffer,
      { useTransaction },
    );
  }

  /**
   * @param {Object} [options]
   * @param {boolean} [options.useTransaction=false]
   * @param {boolean} [options.skipIfExists]
   *
   * @return {Promise<SpentAssetLockTransactionsRepository>}
   */
  async createTree(options = {}) {
    await this.storage.createTree(
      [SpentAssetLockTransactionsRepository.TREE_PATH[0]],
      SpentAssetLockTransactionsRepository.TREE_PATH[1],
      options,
    );

    return this;
  }
}

SpentAssetLockTransactionsRepository.TREE_PATH = [
  Buffer.from([3]),
  Buffer.from([0]),
];

module.exports = SpentAssetLockTransactionsRepository;
