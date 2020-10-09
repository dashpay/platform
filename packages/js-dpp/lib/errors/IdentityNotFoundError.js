const ConsensusError = require('./ConsensusError');

class IdentityNotFoundError extends ConsensusError {
  /**
   * @param {Buffer} identityId
   */
  constructor(identityId) {
    super('Identity not found');

    this.identityId = identityId;
  }

  /**
   * Get identity id
   *
   * @return {Buffer}
   */
  getIdentityId() {
    return this.identityId;
  }
}

module.exports = IdentityNotFoundError;
