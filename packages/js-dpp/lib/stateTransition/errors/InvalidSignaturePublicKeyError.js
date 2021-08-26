const DPPError = require('../../errors/DPPError');

class InvalidSignaturePublicKeyError extends DPPError {
  /**
   *
   * @param {Buffer} signaturePublicKey
   */
  constructor(signaturePublicKey) {
    super('Invalid signature public key');

    this.publicKey = signaturePublicKey;
  }

  /**
   *
   * @returns {Buffer}
   */
  getSignaturePublicKey() {
    return this.publicKey;
  }
}

module.exports = InvalidSignaturePublicKeyError;
