class AbstractTransactionResult {
  /**
   * @param {Object} result
   * @param {number} height
   * @param {Buffer} transaction
   */
  constructor(result, height, transaction) {
    this.deliverResult = result;
    this.height = height;
    this.transaction = transaction;
  }

  /**
   * Get TX result
   *
   * @return {Object}
   */
  getResult() {
    return this.deliverResult;
  }

  /**
   * Get transaction block height
   *
   * @return {number}
   */
  getHeight() {
    return this.height;
  }

  /**
   * Get transaction
   *
   * @return {Buffer}
   */
  getTransaction() {
    return this.transaction;
  }
}

module.exports = AbstractTransactionResult;
