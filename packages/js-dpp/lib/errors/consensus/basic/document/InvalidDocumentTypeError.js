const AbstractBasicError = require('../AbstractBasicError');

class InvalidDocumentTypeError extends AbstractBasicError {
  /**
   * @param {string} type
   * @param {DataContract} dataContract
   */
  constructor(type, dataContract) {
    super(`Data Contract doesn't define document with type ${type}`);

    this.type = type;
    this.dataContract = dataContract;
  }

  /**
   * Get type
   *
   * @return {string}
   */
  getType() {
    return this.type;
  }

  /**
   * Get Data Contract
   *
   * @return {DataContract}
   */
  getDataContract() {
    return this.dataContract;
  }
}

module.exports = InvalidDocumentTypeError;
