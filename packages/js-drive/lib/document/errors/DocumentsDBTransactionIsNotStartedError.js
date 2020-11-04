class DocumentsDBTransactionIsNotStartedError extends Error {
  constructor() {
    super();

    this.name = this.constructor.name;
    this.message = 'MerkDB transaction is not started';

    Error.captureStackTrace(this, this.constructor);
  }
}

module.exports = DocumentsDBTransactionIsNotStartedError;
