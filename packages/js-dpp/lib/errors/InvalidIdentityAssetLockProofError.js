const ConsensusError = require('./ConsensusError');

class InvalidIdentityAssetLockProofError extends ConsensusError {
  /**
   * @param {string} message
   */
  constructor(message) {
    super(`Invalid asset lock proof: ${message}`);
  }
}

module.exports = InvalidIdentityAssetLockProofError;
