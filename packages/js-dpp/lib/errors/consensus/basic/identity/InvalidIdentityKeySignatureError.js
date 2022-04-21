const AbstractBasicError = require('../AbstractBasicError');

class InvalidIdentityKeySignatureError extends AbstractBasicError {
  /**
   * @param {number} publicKeyId
   */
  constructor(publicKeyId) {
    super(`Identity key ${publicKeyId} has invalid signature`);

    this.publicKeyId = publicKeyId;

    // eslint-disable-next-line prefer-rest-params
    this.setConstructorArguments(arguments);
  }

  /**
   * Get id of public key with signature
   *
   * @return {number}
   */
  getPublicKeyId() {
    return this.publicKeyId;
  }
}

module.exports = InvalidIdentityKeySignatureError;
