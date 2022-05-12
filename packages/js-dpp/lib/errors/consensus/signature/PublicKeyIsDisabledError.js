const AbstractSignatureError = require('./AbstractSignatureError');

class PublicKeyIsDisabledError extends AbstractSignatureError {
  /**
   *
   * @param {number} publicKeyId
   */
  constructor(publicKeyId) {
    super('Public key is disabled');

    this.publicKeyId = publicKeyId;
  }

  /**
   * Get disabled public key ID
   *
   * @return {number}
   */
  getPublicKey() {
    return this.publicKeyId;
  }
}

module.exports = PublicKeyIsDisabledError;
