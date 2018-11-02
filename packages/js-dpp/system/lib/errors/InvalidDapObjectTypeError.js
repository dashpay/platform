class InvalidDapObjectTypeError extends Error {
  constructor(dapContract, type) {
    super();

    this.name = this.constructor.name;
    this.message = `Dap contract ${dapContract.name} doesn't contain type ${type}`;
    this.dapContract = dapContract;
    this.type = type;

    if (Error.captureStackTrace) {
      Error.captureStackTrace(this, this.constructor);
    }
  }
}

module.exports = InvalidDapObjectTypeError;
