const AbstractSignatureError = require('./AbstractSignatureError');

class InvalidIdentityPublicKeyTypeError extends AbstractSignatureError {
  /**
   * @param {number} type
   */
  constructor(type) {
    super(`Invalid identity public key type ${type}`);

    this.type = type;
  }

  /**
   * Get identity public key type
   *
   * @return {number}
   */
  getType() {
    return this.type;
  }
}

module.exports = InvalidIdentityPublicKeyTypeError;
