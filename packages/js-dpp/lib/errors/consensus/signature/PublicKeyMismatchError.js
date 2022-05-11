const AbstractSignatureError = require('./AbstractSignatureError');

class PublicKeyMismatchError extends AbstractSignatureError {
  /**
   *
   * @param {IdentityPublicKey} publicKey
   */
  constructor(publicKey) {
    super('Public key mismatched');

    this.publicKey = publicKey;
  }

  /**
   * Get mismatched public key
   *
   * @return {IdentityPublicKey}
   */
  getPublicKey() {
    return this.publicKey;
  }
}

module.exports = PublicKeyMismatchError;
