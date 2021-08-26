const AbstractBasicError = require('../AbstractBasicError');

class InvalidIdentityAssetLockProofError extends AbstractBasicError {
  /**
   * @param {string} message
   */
  constructor(message) {
    super(`Invalid asset lock proof: ${message}`);
  }
}

module.exports = InvalidIdentityAssetLockProofError;
