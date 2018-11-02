class InvalidObjectTypeError extends Error {
  constructor(contract, type) {
    super();

    this.message = `Dap contract ${contract.name} doesn't contain type ${type}`;

    if (Error.captureStackTrace) {
      Error.captureStackTrace(this, this.constructor);
    }
  }
}

module.exports = InvalidObjectTypeError;
