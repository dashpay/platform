const AbstractBasicError = require('../AbstractBasicError');

class IdentityAssetLockTransactionIsNotFoundError extends AbstractBasicError {
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
