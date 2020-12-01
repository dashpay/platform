const ConsensusError = require('./ConsensusError');

class IdentityAssetLockTransactionOutPointAlreadyExistsError extends ConsensusError {
  /**
   * @param {Buffer} outPoint
   */
  constructor(outPoint) {
    super('Asset lock transaction outPoint already exists');

    this.outPoint = outPoint;
  }

  /**
   * @return {Buffer}
   */
  getOutPoint() {
    return this.outPoint;
  }
}

module.exports = IdentityAssetLockTransactionOutPointAlreadyExistsError;
