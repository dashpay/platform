const ConsensusError = require('./ConsensusError');

class IdentityNotFoundError extends ConsensusError {
  /**
   * @param {EncodedBuffer} identityId
   */
  constructor(identityId) {
    super('Identity not found');

    this.identityId = identityId;
  }

  /**
   * Get identity id
   *
   * @return {EncodedBuffer}
   */
  getIdentityId() {
    return this.identityId;
  }
}

module.exports = IdentityNotFoundError;
