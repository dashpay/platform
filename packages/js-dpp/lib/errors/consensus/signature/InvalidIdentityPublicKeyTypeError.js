const AbstractSignatureError = require('./AbstractSignatureError');

class InvalidIdentityPublicKeyTypeError extends AbstractSignatureError {
  /**
   *
   * @param {number} publicKeyType
   */
  constructor(publicKeyType) {
    super(`Unsupported signature type ${publicKeyType}. Please use type ECDSA (0), BLS (1) or ECDSA HASH160 (2) keys to sign the state transition`);

    this.publicKeyType = publicKeyType;
  }

  /**
   * @returns {number}
   */
  getPublicKeyType() {
    return this.publicKeyType;
  }
}

module.exports = InvalidIdentityPublicKeyTypeError;
