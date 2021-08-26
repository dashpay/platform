const AbstractBasicError = require('../AbstractBasicError');

class InvalidIdentityAssetLockTransactionOutputError extends AbstractBasicError {
  /**
   * @param {string} message
   * @param {Object} output
   */
  constructor(message, output) {
    super(`Invalid asset lock transaction output: ${message}`);

    this.output = output;
  }

  /**
   * Get lock transaction output
   *
   * @return {Object}
   */
  getOutput() {
    return this.output;
  }
}

module.exports = InvalidIdentityAssetLockTransactionOutputError;
