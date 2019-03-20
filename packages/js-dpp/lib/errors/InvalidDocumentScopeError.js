const ConsensusError = require('./ConsensusError');

class InvalidDocumentScopeError extends ConsensusError {
  /**
   * @param {Document} document
   */
  constructor(document) {
    super('Invalid Document scope');

    this.document = document;
  }

  /**
   * Get Document
   *
   * @return {Document}
   */
  getDocument() {
    return this.document;
  }
}

module.exports = InvalidDocumentScopeError;
