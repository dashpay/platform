const ConsensusError = require('./ConsensusError');

class InvalidDocumentScopeIdError extends ConsensusError {
  /**
   * @param {Object} rawDocument
   */
  constructor(rawDocument) {
    super('Invalid Document scopeId');

    this.rawDocument = rawDocument;
  }

  /**
   * Get raw Document
   *
   * @return {Object}
   */
  getRawDocument() {
    return this.rawDocument;
  }
}

module.exports = InvalidDocumentScopeIdError;
