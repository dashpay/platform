const AbstractBasicError = require('../AbstractBasicError');

class InvalidAssetLockTransactionOutputReturnSize extends AbstractBasicError {
  /**
   * @param {number} outputIndex
   */
  constructor(outputIndex) {
    super(`Asset Lock output ${outputIndex} has invalid public key hash. Must be 20 length bytes hash.`);

    this.outputIndex = outputIndex;

    // eslint-disable-next-line prefer-rest-params
    this.setConstructorArguments(arguments);
  }

  /**
   * Get asset lock transaction output index
   *
   * @return {number}
   */
  getOutputIndex() {
    return this.outputIndex;
  }
}

module.exports = InvalidAssetLockTransactionOutputReturnSize;
