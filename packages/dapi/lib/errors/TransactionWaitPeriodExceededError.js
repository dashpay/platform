class TransactionWaitPeriodExceededError extends Error {
  /**
   * @param {string} transactionHash
   */
  constructor(transactionHash) {
    const message = `Transaction waiting period for ${transactionHash} exceeded`;
    super(message);

    this.transactionHash = transactionHash;
  }

  /**
   * Returns transaction hash
   * @return {string}
   */
  getTransactionHash() {
    return this.transactionHash;
  }
}

module.exports = TransactionWaitPeriodExceededError;
