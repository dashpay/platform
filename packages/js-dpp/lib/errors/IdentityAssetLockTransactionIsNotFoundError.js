const ConsensusError = require('./ConsensusError');

class IdentityAssetLockTransactionIsNotFoundError extends ConsensusError {
  /**
   *
   * @param {Buffer} outPoint
   */
  constructor(outPoint) {
    super('Asset Lock transaction with specified outPoint was not found');

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

module.exports = IdentityAssetLockTransactionIsNotFoundError;
