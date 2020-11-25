const ConsensusError = require('./ConsensusError');

class InvalidIdentityAssetLockTransactionError extends ConsensusError {
  /**
   * @param {string} message
   */
  constructor(message) {
    super(`Invalid asset lock transaction: ${message}`);
  }
}

module.exports = InvalidIdentityAssetLockTransactionError;
