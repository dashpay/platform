const ConsensusError = require('./ConsensusError');

class MissingPublicKeyError extends ConsensusError {
  constructor(publicKeyId) {
    super("Public key with such id doesn't exist");

    this.publicKeyId = publicKeyId;
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
