const DPPError = require('../../errors/DPPError');

class NoDocumentsSuppliedError extends DPPError {
  constructor() {
    super('No documents were supplied to state transition');
  }
}

module.exports = NoDocumentsSuppliedError;
