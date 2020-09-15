const ConsensusError = require('./ConsensusError');

class IdentityPublicKeyAlreadyExistsError extends ConsensusError {
  /**
   * @param {string} publicKeyHash
   */
  constructor(publicKeyHash) {
    super('Identity public key already exists');

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

module.exports = IdentityPublicKeyAlreadyExistsError;
