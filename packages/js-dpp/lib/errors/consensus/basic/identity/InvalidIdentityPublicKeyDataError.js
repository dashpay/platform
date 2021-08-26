const AbstractBasicError = require('../AbstractBasicError');

class InvalidIdentityPublicKeyDataError extends AbstractBasicError {
  /**
   * @param {RawIdentityPublicKey} publicKey
   * @param {Error} validationError
   */
  constructor(publicKey, validationError) {
    super(`Invalid identity public key data ${publicKey.data}`);

    this.publicKey = publicKey;
    this.validationError = validationError;
  }

  /**
   * Get identity public key
   *
   * @return {RawIdentityPublicKey}
   */
  getPublicKey() {
    return this.publicKey;
  }

  /**
   * Get public key data validation error
   *
   * @return {Error}
   */
  getValidationError() {
    return this.validationError;
  }
}

module.exports = InvalidIdentityPublicKeyDataError;
