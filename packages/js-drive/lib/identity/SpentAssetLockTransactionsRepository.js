class SpentAssetLockTransactionsRepository {
  /**
   * @param {MerkDbStore} spentAssetLockTransactionsStore
   */
  constructor(spentAssetLockTransactionsStore) {
    this.storage = spentAssetLockTransactionsStore;
  }

  /**
   * Store the outPoint
   *
   * @param {Buffer} outPointBuffer
   * @param {MerkDbTransaction} [transaction]
   *
   * @return {SpentAssetLockTransactionsRepository}
   */
  store(outPointBuffer, transaction = undefined) {
    this.storage.put(
      outPointBuffer,
      Buffer.from([1]),
      transaction,
    );

    return this;
  }

  /**
   * Fetch the outPoint
   *
   * @param {Buffer} outPointBuffer
   * @param {MerkDbTransaction} [transaction]
   *
   * @return {null|Buffer}
   */
  fetch(outPointBuffer, transaction = undefined) {
    const result = this.storage.get(outPointBuffer, transaction);

    if (!result) {
      return null;
    }

    return result;
  }
}

module.exports = SpentAssetLockTransactionsRepository;
