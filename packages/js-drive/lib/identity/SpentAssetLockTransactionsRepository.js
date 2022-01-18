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
    const result = await this.storage.get(
      SpentAssetLockTransactionsRepository.TREE_PATH,
      outPointBuffer,
      { useTransaction },
    );

    if (!result) {
      return null;
    }

    return result;
  }

  /**
   * @return {Promise<SpentAssetLockTransactionsRepository>}
   */
  async createTree() {
    await this.storage.createTree([], SpentAssetLockTransactionsRepository.TREE_PATH[0]);

    return this;
  }
}

SpentAssetLockTransactionsRepository.TREE_PATH = [
  Buffer.from('misc'),
  Buffer.from('spentAssetLockTransactions'),
];

module.exports = SpentAssetLockTransactionsRepository;
