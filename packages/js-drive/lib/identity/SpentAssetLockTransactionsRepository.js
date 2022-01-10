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
   * @param {GroveDBTransaction} [transaction]
   *
   * @return {SpentAssetLockTransactionsRepository}
   */
  async store(outPointBuffer, transaction = undefined) {
    await this.storage.put(
      SpentAssetLockTransactionsRepository.TREE_PATH,
      outPointBuffer,
      Buffer.from([1]),
      { transaction },
    );

    return this;
  }

  /**
   * Fetch the outPoint
   *
   * @param {Buffer} outPointBuffer
   * @param {GroveDBTransaction} [transaction]
   *
   * @return {null|Buffer}
   */
  async fetch(outPointBuffer, transaction = undefined) {
    const result = await this.storage.get(
      SpentAssetLockTransactionsRepository.TREE_PATH,
      outPointBuffer,
      { transaction },
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
