const AbstractSignatureError = require('./AbstractSignatureError');

class InvalidStateTransitionSignatureError extends AbstractSignatureError {
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

module.exports = InvalidStateTransitionSignatureError;
