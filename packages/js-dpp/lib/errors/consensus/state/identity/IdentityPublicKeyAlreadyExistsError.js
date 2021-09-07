const AbstractStateError = require('../AbstractStateError');

class IdentityPublicKeyAlreadyExistsError extends AbstractStateError {
  /**
   * @param {Buffer} publicKeyHash
   */
  constructor(publicKeyHash) {
    super(`Identity public key ${publicKeyHash.toString('hex')} already exists`);

    this.publicKeyHash = publicKeyHash;

    // eslint-disable-next-line prefer-rest-params
    this.setConstructorArguments(arguments);
  }

  /**
   * Get public key hash
   *
   * @return {Buffer}
   */
  getPublicKeyHash() {
    return this.publicKeyHash;
  }
}

module.exports = IdentityPublicKeyAlreadyExistsError;
