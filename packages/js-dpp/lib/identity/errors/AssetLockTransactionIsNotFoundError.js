const DPPError = require('../../errors/DPPError');

class AssetLockTransactionIsNotFoundError extends DPPError {
  /**
   * @param {string} transactionId
   */
  constructor(transactionId) {
    super(`Asset Lock transaction ${transactionId} is not found`);

    this.transactionId = transactionId;
  }

  /**
   *
   * @returns {string}
   */
  getTransactionId() {
    return this.transactionId;
  }
}

module.exports = AssetLockTransactionIsNotFoundError;
