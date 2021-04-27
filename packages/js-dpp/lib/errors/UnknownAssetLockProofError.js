const ConsensusError = require('./ConsensusError');

class UnknownAssetLockProofError extends ConsensusError {
  /**
   *
   * @param {number} type
   */
  constructor(type) {
    super('Unknown Asset lock proof type');

    this.type = type;
  }

  /**
   *
   * @returns {number}
   */
  getType() {
    return this.type;
  }
}

module.exports = UnknownAssetLockProofError;
