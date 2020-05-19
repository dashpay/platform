const ConsensusError = require('./ConsensusError');

class IdentityLockTransactionIsNotFinalizedError extends ConsensusError {
  /**
   * @param {string} transactionHash
   */
  constructor(transactionHash) {
    super('Identity lock transaction is not finalized');

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

module.exports = IdentityLockTransactionIsNotFinalizedError;
