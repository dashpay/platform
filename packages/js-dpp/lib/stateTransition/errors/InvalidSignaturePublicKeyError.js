class InvalidSignaturePublicKeyError extends Error {
  /**
   *
   * @param {EncodedBuffer} signaturePublicKey
   */
  constructor(signaturePublicKey) {
    super();

    this.name = this.constructor.name;
    this.message = 'Invalid signature public key';
    this.publicKey = signaturePublicKey;
  }

  /**
   *
   * @returns {EncodedBuffer}
   */
  getSignaturePublicKey() {
    return this.publicKey;
  }
}

module.exports = InvalidSignaturePublicKeyError;
