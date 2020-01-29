const ConsensusError = require('./ConsensusError');

class IdentityNotFoundError extends ConsensusError {
  /**
   * @param {string} identityId
   */
  constructor(identityId) {
    super('Identity not found');

    this.identityId = identityId;
  }

  /**
   * Get identity id
   *
   * @return {string}
   */
  getIdentityId() {
    return this.identityId;
  }
}

module.exports = IdentityNotFoundError;
