const ConsensusError = require('./ConsensusError');

class InvalidDPObjectTypeError extends ConsensusError {
  /**
   * @param {string} type
   * @param {DPContract} dpContract
   */
  constructor(type, dpContract) {
    super(`DP contract ${dpContract.name} doesn't contain type ${type}`);

    this.name = this.constructor.name;

    this.type = type;
    this.dpContract = dpContract;

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
   * Get DP Contract
   *
   * @return {DPContract}
   */
  getDPContract() {
    return this.dpContract;
  }
}

module.exports = InvalidDPObjectTypeError;
