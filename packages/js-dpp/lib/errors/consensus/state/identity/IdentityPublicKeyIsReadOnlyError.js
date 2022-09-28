const AbstractStateError = require('../AbstractStateError');

class IdentityPublicKeyIsReadOnlyError extends AbstractStateError {
  /**
   * @param {number} publicKeyIndex
   */
  constructor(publicKeyIndex) {
    super(`Identity Public Key #${publicKeyIndex} is read only`);

    this.publicKeyIndex = publicKeyIndex;
  }

  /**
   *
   * @returns {number}
   */
  getPublicKeyIndex() {
    return this.publicKeyIndex;
  }
}

module.exports = IdentityPublicKeyIsReadOnlyError;
