const AbstractBasicError = require('../AbstractBasicError');

class IdentityAssetLockTransactionIsNotFoundError extends AbstractBasicError {
  /**
   * @param {Buffer} transactionId
   */
  constructor(transactionId) {
    super(`Asset Lock transaction ${transactionId.toString('hex')} is not found`);

    this.transactionId = transactionId;

    // eslint-disable-next-line prefer-rest-params
    this.setConstructorArguments(arguments);
  }

  /**
   *
   * @returns {Buffer}
   */
  getTransactionId() {
    return this.transactionId;
  }
}

module.exports = IdentityAssetLockTransactionIsNotFoundError;
