const AbstractBasicError = require('../AbstractBasicError');

class InvalidIdentityAssetLockTransactionError extends AbstractBasicError {
  /**
   * @param {string} message
   */
  constructor(message) {
    super(`Invalid asset lock transaction: ${message}`);

    // eslint-disable-next-line prefer-rest-params
    this.setConstructorArguments(arguments);
  }

  /**
   * @param {Error} error
   */
  setValidationError(error) {
    this.validationError = error;
  }

  /**
   * @returns {Error}
   */
  getValidationError() {
    return this.validationError;
  }
}

module.exports = InvalidIdentityAssetLockTransactionError;
