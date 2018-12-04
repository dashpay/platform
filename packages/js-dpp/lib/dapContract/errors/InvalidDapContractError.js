class InvalidDapContractError extends Error {
  /**
   * @param {ConsensusError[]} errors
   * @param {Object} rawDapContract
   */
  constructor(errors, rawDapContract) {
    super();

    this.name = this.constructor.name;
    this.message = 'Invalid Dap Contract';

    this.errors = errors;
    this.rawDapContract = rawDapContract;

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
   * Get raw Dap Contract
   *
   * @return {Object}
   */
  getRawDapContract() {
    return this.rawDapContract;
  }
}

module.exports = InvalidDapContractError;
