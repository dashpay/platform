const AbstractSignatureError = require('./AbstractSignatureError');

class PublicKeyIsDisabledError extends AbstractSignatureError {
  /**
   *
   * @param {IdentityPublicKey} publicKey
   */
  constructor(publicKey) {
    super('Public key is disabled');

    this.publicKey = publicKey;
  }

  /**
   * Get disabled public key
   *
   * @return {IdentityPublicKey}
   */
  getPublicKey() {
    return this.publicKey;
  }
}

module.exports = PublicKeyIsDisabledError;
