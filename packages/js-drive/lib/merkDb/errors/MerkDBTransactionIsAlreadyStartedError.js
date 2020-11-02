class MerkDBTransactionIsAlreadyStartedError extends Error {
  /**
   * Indicates, if Transaction was started when it should't
   */
  constructor() {
    super();

    this.name = this.constructor.name;
    this.message = 'MerkDB transaction is already started';

    Error.captureStackTrace(this, this.constructor);
  }
}

module.exports = MerkDBTransactionIsAlreadyStartedError;
