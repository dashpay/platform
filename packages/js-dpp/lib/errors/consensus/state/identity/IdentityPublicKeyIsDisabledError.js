const AbstractStateError = require('../AbstractStateError');

class IdentityPublicKeyIsDisabledError extends AbstractStateError {
  /**
   * @param {number} publicKeyIndex
   */
  constructor(publicKeyIndex) {
    super(`Identity Public Key #${publicKeyIndex} is disabled`);

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

module.exports = IdentityPublicKeyIsDisabledError;
