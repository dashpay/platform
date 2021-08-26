const AbstractSignatureError = require('./AbstractSignatureError');

class MissingPublicKeyError extends AbstractSignatureError {
  /**
   * @param {number} publicKeyId
   */
  constructor(publicKeyId) {
    super("Public key with such id doesn't exist");

    this.publicKeyId = publicKeyId;
  }

  /**
   * Get public key id
   *
   * @return {number}
   */
  getPublicKeyId() {
    return this.publicKeyId;
  }
}

module.exports = MissingPublicKeyError;
