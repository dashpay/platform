const ConsensusError = require('./ConsensusError');

class InvalidIdentityPublicKeyTypeError extends ConsensusError {
  /**
   * @param {number} type
   */
  constructor(type) {
    super(`Invalid identity public key type ${type}`);

    this.type = type;
  }

  /**
   * Get identity public key type
   *
   * @return {number}
   */
  getType() {
    return this.type;
  }
}

module.exports = InvalidIdentityPublicKeyTypeError;
