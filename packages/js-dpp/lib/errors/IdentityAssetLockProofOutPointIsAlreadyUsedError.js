const ConsensusError = require('./ConsensusError');

class IdentityAssetLockProofOutPointIsAlreadyUsedError extends ConsensusError {
  /**
   *
   * @param {Buffer} outPoint
   */
  constructor(outPoint) {
    super('Asset lock proof outPoint was already used');

    this.outPoint = outPoint;
  }

  /**
   *
   * @returns {Buffer}
   */
  getOutPoint() {
    return this.outPoint;
  }
}

module.exports = IdentityAssetLockProofOutPointIsAlreadyUsedError;
