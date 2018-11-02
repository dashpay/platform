class ContractIdIsNotDefinedError extends Error {
  constructor() {
    super();

    this.message = 'Contract ID is not defined';

    if (Error.captureStackTrace) {
      Error.captureStackTrace(this, this.constructor);
    }
  }
}

module.exports = ContractIdIsNotDefinedError;
