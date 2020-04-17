class NoDocumentsSuppliedError extends Error {
  constructor() {
    super();

    this.name = this.constructor.name;
    this.message = 'No documents were supplied to state transition';
  }
}

module.exports = NoDocumentsSuppliedError;
