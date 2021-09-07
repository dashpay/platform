const AbstractSignatureError = require('./AbstractSignatureError');

class MissingPublicKeyError extends AbstractSignatureError {
  /**
   * @param {number} publicKeyId
   */
  constructor(publicKeyId) {
    super(`Public key ${publicKeyId} doesn't exist`);

    this.publicKeyId = publicKeyId;

    // eslint-disable-next-line prefer-rest-params
    this.setConstructorArguments(arguments);
  }

  /**
   * Get public key id
   *
   * @return {number}
   */
  getPublicKeyId() {
    return this.publicKeyId;
  }
}

module.exports = MissingPublicKeyError;
