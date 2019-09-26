class TransactionIsNotStartedError extends Error {
  /**
   * Indicates, if Transaction was not started when is should
   */
  constructor() {
    super();

    this.name = this.constructor.name;
    this.message = 'Transaction is not started';

    Error.captureStackTrace(this, this.constructor);
  }
}

module.exports = TransactionIsNotStartedError;
