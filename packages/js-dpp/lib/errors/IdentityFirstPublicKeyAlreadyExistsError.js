const ConsensusError = require('./ConsensusError');

class IdentityFirstPublicKeyAlreadyExistsError extends ConsensusError {
  /**
   * @param {string} publicKeyHash
   */
  constructor(publicKeyHash) {
    super('Identity first public key already exists');

    this.publicKeyHash = publicKeyHash;
  }

  /**
   * Get public key hash
   *
   * @return {string}
   */
  getPublicKeyHash() {
    return this.publicKeyHash;
  }
}

module.exports = IdentityFirstPublicKeyAlreadyExistsError;
