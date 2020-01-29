class InvalidSignaturePublicKeyError extends Error {
  /**
   *
   * @param {string} signaturePublicKey
   */
  constructor(signaturePublicKey) {
    super();

    this.name = this.constructor.name;
    this.message = 'Invalid signature public key';
    this.publicKey = signaturePublicKey;
  }

  /**
   *
   * @returns {string}
   */
  getSignaturePublicKey() {
    return this.publicKey;
  }
}

module.exports = InvalidSignaturePublicKeyError;
