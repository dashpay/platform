class InvalidContractError extends Error {
  /**
   * @param {ConsensusError[]} errors
   * @param {RawDocument} rawContract
   */
  constructor(errors, rawContract) {
    super();

    this.name = this.constructor.name;
    this.message = 'Invalid Contract';

    this.errors = errors;
    this.rawContract = rawContract;

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
   * Get raw Contract
   *
   * @return {RawContract}
   */
  getRawContract() {
    return this.rawContract;
  }
}

module.exports = InvalidContractError;
