class InvalidDapObjectTypeError extends Error {
  /**
   * @param {string} type
   * @param {DapContract} dapContract
   */
  constructor(type, dapContract) {
    super();

    this.name = this.constructor.name;
    this.message = `Dap contract ${dapContract.name} doesn't contain type ${type}`;

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
