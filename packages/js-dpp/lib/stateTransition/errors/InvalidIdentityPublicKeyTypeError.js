const DPPError = require('../../errors/DPPError');

class InvalidIdentityPublicKeyTypeError extends DPPError {
  /**
   *
   * @param {number} publicKeyType
   */
  constructor(publicKeyType) {
    super('Invalid signature type');

    this.publicKeyType = publicKeyType;
  }

  /**
   * @returns {number}
   */
  getPublicKeyType() {
    return this.publicKeyType;
  }
}

module.exports = InvalidIdentityPublicKeyTypeError;
