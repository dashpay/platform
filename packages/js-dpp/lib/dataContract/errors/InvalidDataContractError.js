class InvalidDataContractError extends Error {
  /**
   * @param {ConsensusError[]} errors
   * @param {RawDataContract} rawDataContract
   */
  constructor(errors, rawDataContract) {
    super();

    this.name = this.constructor.name;
    this.message = 'Invalid Data Contract';

    this.errors = errors;
    this.rawDataContract = rawDataContract;

    if (Error.captureStackTrace) {
      Error.captureStackTrace(this, this.constructor);
    }
  }

  /**
   * Get validation errors
   *
   * @return {ConsensusError[]}
   */
  getErrors() {
    return this.errors;
  }

  /**
   * Get raw Data Contract
   *
   * @return {RawDataContract}
   */
  getRawDataContract() {
    return this.rawDataContract;
  }
}

module.exports = InvalidDataContractError;
