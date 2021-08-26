const DPPError = require('../../errors/DPPError');

class PublicKeyMismatchError extends DPPError {
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
