class AbstractTransactionResult {
  /**
   * @param {Object} deliverResult
   * @param {Buffer} transaction
   */
  constructor(deliverResult, transaction) {
    this.deliverResult = deliverResult;
    this.transaction = transaction;
  }

  /**
   * Get TX result
   *
   * @return {Object}
   */
  getDeliverResult() {
    return this.deliverResult;
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
