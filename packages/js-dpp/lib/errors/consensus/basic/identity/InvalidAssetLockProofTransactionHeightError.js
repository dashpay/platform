const AbstractBasicError = require('../AbstractBasicError');

class InvalidAssetLockProofTransactionHeightError extends AbstractBasicError {
  /**
   * @param {number} proofCoreChainLockedHeight
   * @param {number} transactionHeight
   */
  constructor(proofCoreChainLockedHeight, transactionHeight) {
    super(`Core chain locked height ${proofCoreChainLockedHeight} must be higher than block ${transactionHeight || ''} with Asset Lock transaction`);

    this.proofCoreChainLockedHeight = proofCoreChainLockedHeight;
    this.transactionHeight = transactionHeight;

    // eslint-disable-next-line prefer-rest-params
    this.setConstructorArguments(arguments);
  }

  /**
   *
   * @returns {number}
   */
  getProofCoreChainLockedHeight() {
    return this.proofCoreChainLockedHeight;
  }

  /**
   *
   * @returns {number}
   */
  getTransactionHeight() {
    return this.transactionHeight;
  }
}

module.exports = InvalidAssetLockProofTransactionHeightError;
