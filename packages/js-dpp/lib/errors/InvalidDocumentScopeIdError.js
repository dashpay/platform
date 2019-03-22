const ConsensusError = require('./ConsensusError');

class InvalidDocumentScopeIdError extends ConsensusError {
  /**
   * @param {RawDocument} rawDocument
   */
  constructor(rawDocument) {
    super('Invalid Document scopeId');

    this.rawDocument = rawDocument;
  }

  /**
   * Get raw Document
   *
   * @return {RawDocument}
   */
  getRawDocument() {
    return this.rawDocument;
  }
}

module.exports = InvalidDocumentScopeIdError;
