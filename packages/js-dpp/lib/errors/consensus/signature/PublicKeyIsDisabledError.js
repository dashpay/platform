const AbstractSignatureError = require('./AbstractSignatureError');

class PublicKeyIsDisabledError extends AbstractSignatureError {
  /**
   *
   * @param {number} publicKeyId
   */
  constructor(publicKeyId) {
    super(`Identity key ${publicKeyId} is disabled`);

    this.publicKeyId = publicKeyId;
  }

  /**
   * Get disabled public key ID
   *
   * @return {number}
   */
  getPublicKeyId() {
    return this.publicKeyId;
  }
}

module.exports = PublicKeyIsDisabledError;
