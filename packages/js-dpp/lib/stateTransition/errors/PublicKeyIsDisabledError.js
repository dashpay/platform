const DPPError = require('../../errors/DPPError');

class PublicKeyIsDisabledError extends DPPError {
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
