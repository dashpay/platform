const ConsensusError = require('./ConsensusError');

class InvalidDapObjectTypeError extends ConsensusError {
  /**
   * @param {string} type
   * @param {DapContract} dapContract
   */
  constructor(type, dapContract) {
    super(`Dap contract ${dapContract.name} doesn't contain type ${type}`);

    this.name = this.constructor.name;

    this.type = type;
    this.dapContract = dapContract;

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
   * Get Dap Contract
   *
   * @return {DapContract}
   */
  getDapContract() {
    return this.dapContract;
  }
}

module.exports = InvalidDapObjectTypeError;
