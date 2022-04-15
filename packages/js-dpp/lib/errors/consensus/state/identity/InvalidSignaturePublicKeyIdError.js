const AbstractStateError = require('../AbstractStateError');

class InvalidSignaturePublicKeyIdError extends AbstractStateError {
  /**
   * @param {number} id
   */
  constructor(id) {
    super(`Identity public key with ID ${id} does not suitable for transition with script signature`);

    this.publicKeyId = id;

    // eslint-disable-next-line prefer-rest-params
    this.setConstructorArguments(arguments);
  }

  /**
   * Get Identity Public Key ID
   *
   * @return {number}
   */
  getPublicKeyId() {
    return this.publicKeyId;
  }
}

module.exports = InvalidSignaturePublicKeyIdError;
