class TransactionWaitPeriodExceededError extends Error {
  /**
   * @param {string} transactionHash
   * @param originalStack
   */
  constructor(transactionHash, originalStack) {
    const message = `Transaction waiting period for ${transactionHash} exceeded`;
    super(message);
    if (originalStack) {
      this.stack = originalStack;
    }

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
