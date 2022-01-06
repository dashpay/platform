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
   * @param {GroveDBTransaction} [transaction]
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
