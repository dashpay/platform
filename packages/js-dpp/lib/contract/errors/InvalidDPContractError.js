class InvalidDPContractError extends Error {
  /**
   * @param {ConsensusError[]} errors
   * @param {Object} rawDPContract
   */
  constructor(errors, rawDPContract) {
    super();

    this.name = this.constructor.name;
    this.message = 'Invalid DP Contract';

    this.errors = errors;
    this.rawDPContract = rawDPContract;

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
   * Get raw DP Contract
   *
   * @return {Object}
   */
  getRawDPContract() {
    return this.rawDPContract;
  }
}

module.exports = InvalidDPContractError;
