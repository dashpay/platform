const ConsensusError = require('./ConsensusError');

class IdentityLockTransactionNotFoundError extends ConsensusError {
  /**
   * @param {string} transactionHash
   */
  constructor(transactionHash) {
    super('Identity lock transaction not found');

    this.transactionHash = transactionHash;
  }

  /**
   * Get transaction hash
   *
   * @return {string}
   */
  getTransactionHash() {
    return this.transactionHash;
  }
}

module.exports = IdentityLockTransactionNotFoundError;
