const ConsensusError = require('./ConsensusError');

class IdentityAssetLockTransactionNotFoundError extends ConsensusError {
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

module.exports = IdentityAssetLockTransactionNotFoundError;
