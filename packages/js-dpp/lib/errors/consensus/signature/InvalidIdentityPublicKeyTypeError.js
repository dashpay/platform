const AbstractSignatureError = require('./AbstractSignatureError');

class InvalidIdentityPublicKeyTypeError extends AbstractSignatureError {
  /**
   *
   * @param {number} publicKeyType
   */
  constructor(publicKeyType) {
    super('Invalid signature type');

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
