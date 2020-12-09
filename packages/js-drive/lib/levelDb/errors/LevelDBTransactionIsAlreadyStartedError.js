class LevelDBTransactionIsAlreadyStartedError extends Error {
  /**
   * Indicates, if Transaction was started when it shouldn't
   */
  constructor() {
    super();

    this.name = this.constructor.name;
    this.message = 'LevelDB transaction is already started';

    Error.captureStackTrace(this, this.constructor);
  }
}

module.exports = LevelDBTransactionIsAlreadyStartedError;
