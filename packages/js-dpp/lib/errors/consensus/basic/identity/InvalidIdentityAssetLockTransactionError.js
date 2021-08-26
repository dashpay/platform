const AbstractBasicError = require('../AbstractBasicError');

class InvalidIdentityAssetLockTransactionError extends AbstractBasicError {
  /**
   * @param {string} message
   */
  constructor(message) {
    super(`Invalid asset lock transaction: ${message}`);
  }
}

module.exports = InvalidIdentityAssetLockTransactionError;
