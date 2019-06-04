class TransactionHashesCache {
  constructor() {
    this.transactionHashes = {};
    this.cacheSize = 10;
  }

  /**
   * Add a transaction hash
   *
   * @param {string} transactionHash
   */
  add(transactionHash) {
    this.transactionHashes[transactionHash] = 0;
  }

  /**
   * Get confirmations count for specified transaction
   *
   * @param {string} transactionHash
   * @return {number}
   */
  getConfirmationsCount(transactionHash) {
    return this.transactionHashes[transactionHash];
  }

  /**
   * Remove transactions from cache if they have enough confirmations
   *
   * @param {string[]} blockTransactionHashes
   */
  updateByBlockTransactionHashes(blockTransactionHashes) {
    Object.entries(this.transactionHashes).forEach(([hash, confirmationsCount]) => {
      if (blockTransactionHashes.includes(hash)) {
        // Set confirmations count to 1 if block contains the transaction
        this.transactionHashes[hash] = 1;
      } else if (confirmationsCount >= 1) {
        // Increment confirmations count if a new block added on top of it
        this.transactionHashes[hash] = confirmationsCount + 1;

        // Remove the transaction if more than N blocks added on top of it
        if (confirmationsCount >= this.cacheSize) {
          delete this.transactionHashes[hash];
        }
      }
    });
  }
}

module.exports = TransactionHashesCache;
