class InvalidSignatureTypeError extends Error {
  /**
   *
   * @param {number} signatureType
   */
  constructor(signatureType) {
    super();

    this.name = this.constructor.name;
    this.message = 'Invalid signature type';
    this.signatureType = signatureType;
  }

  /**
   *
   * @returns {number}
   */
  getSignatureType() {
    return this.signatureType;
  }
}

module.exports = InvalidSignatureTypeError;
