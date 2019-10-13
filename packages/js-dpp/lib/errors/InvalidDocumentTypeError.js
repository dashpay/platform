const ConsensusError = require('./ConsensusError');

class InvalidDocumentTypeError extends ConsensusError {
  /**
   * @param {string} type
   * @param {DataContract} dataContract
   */
  constructor(type, dataContract) {
    super(`Contract doesn't contain type ${type}`);

    this.name = this.constructor.name;

    this.type = type;
    this.dataContract = dataContract;

    if (Error.captureStackTrace) {
      Error.captureStackTrace(this, this.constructor);
    }
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
