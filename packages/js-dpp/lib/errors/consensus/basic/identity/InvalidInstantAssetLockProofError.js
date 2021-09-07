const AbstractBasicError = require('../AbstractBasicError');

class InvalidInstantAssetLockProofError extends AbstractBasicError {
  /**
   * @param {string} message
   */
  constructor(message) {
    super(`Invalid instant lock proof: ${message}`);

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
   * @return {Error}
   */
  getValidationError() {
    return this.validationError;
  }
}

module.exports = InvalidInstantAssetLockProofError;
