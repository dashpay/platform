const AbstractBasicError = require('../AbstractBasicError');

class IdentityAssetLockTransactionOutPointAlreadyExistsError extends AbstractBasicError {
  /**
   * @param {Buffer} transactionId
   * @param {number} outputIndex
   */
  constructor(transactionId, outputIndex) {
    super(`Asset lock transaction ${transactionId.toString('hex')} output ${outputIndex} already used`);

    this.transactionId = transactionId;
    this.outputIndex = outputIndex;

    // eslint-disable-next-line prefer-rest-params
    this.setConstructorArguments(arguments);
  }

  /**
   * @return {Buffer}
   */
  getTransactionId() {
    return this.transactionId;
  }

  /**
   * @return {number}
   */
  getOutputIndex() {
    return this.outputIndex;
  }
}

module.exports = IdentityAssetLockTransactionOutPointAlreadyExistsError;
