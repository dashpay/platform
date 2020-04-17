class InvalidDataContractError extends Error {
  /**
   * @param {ConsensusError[]} errors
   * @param {RawDataContract} rawDataContract
   */
  constructor(errors, rawDataContract) {
    super();

    this.name = this.constructor.name;
    this.message = `Invalid Data Contract: "${errors[0].message}"`;
    if (errors.length > 1) {
      this.message = `${this.message} and ${errors.length - 1} more`;
    }

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
