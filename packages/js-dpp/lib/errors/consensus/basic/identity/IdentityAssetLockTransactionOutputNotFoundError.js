const AbstractBasicError = require('../AbstractBasicError');

class IdentityAssetLockTransactionOutputNotFoundError extends AbstractBasicError {
  /**
   * @param {number} outputIndex
   */
  constructor(outputIndex) {
    super(`Asset Lock Transaction Output with index ${outputIndex} not found`);

    this.outputIndex = outputIndex;
  }

  /**
   * @return {number}
   */
  getOutputIndex() {
    return this.outputIndex;
  }
}

module.exports = IdentityAssetLockTransactionOutputNotFoundError;
