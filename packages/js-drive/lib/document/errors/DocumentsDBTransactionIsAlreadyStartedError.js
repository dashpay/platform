class DocumentsDBTransactionIsAlreadyStartedError extends Error {
  constructor() {
    super();

    this.name = this.constructor.name;
    this.message = 'Documents DB transaction is already started';

    Error.captureStackTrace(this, this.constructor);
  }
}

module.exports = DocumentsDBTransactionIsAlreadyStartedError;
