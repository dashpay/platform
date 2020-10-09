class InvalidSignaturePublicKeyError extends Error {
  /**
   *
   * @param {Buffer} signaturePublicKey
   */
  constructor(signaturePublicKey) {
    super();

    this.name = this.constructor.name;
    this.message = 'Invalid signature public key';
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
