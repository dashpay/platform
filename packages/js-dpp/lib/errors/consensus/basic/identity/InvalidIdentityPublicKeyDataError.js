const AbstractBasicError = require('../AbstractBasicError');

class InvalidIdentityPublicKeyDataError extends AbstractBasicError {
  /**
   * @param {number} publicKeyId
   * @param {string} message
   */
  constructor(publicKeyId, message) {
    super(`Invalid identity public key ${publicKeyId} data: ${message}`);

    this.publicKeyId = publicKeyId;

    // eslint-disable-next-line prefer-rest-params
    this.setConstructorArguments(arguments);
  }

  /**
   * Get identity public key ID
   *
   * @return {number}
   */
  getPublicKeyId() {
    return this.publicKeyId;
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

module.exports = InvalidIdentityPublicKeyDataError;
