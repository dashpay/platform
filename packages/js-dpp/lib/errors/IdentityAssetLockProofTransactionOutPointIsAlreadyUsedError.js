const ConsensusError = require('./ConsensusError');

class IdentityAssetLockProofTransactionOutPointIsAlreadyUsedError extends ConsensusError {
  /**
   *
   * @param {Buffer} outPoint
   */
  constructor(outPoint) {
    super('Asset lock proof transaction outPoint was already used');

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

module.exports = IdentityAssetLockProofTransactionOutPointIsAlreadyUsedError;
