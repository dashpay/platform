const ConsensusError = require('./ConsensusError');

class InvalidIdentityAssetLockTransactionOutPointError extends ConsensusError {
  /**
   * @param {string} message
   */
  constructor(message) {
    super(`Invalid Identity out point: ${message}`);
  }
}

module.exports = InvalidIdentityAssetLockTransactionOutPointError;
