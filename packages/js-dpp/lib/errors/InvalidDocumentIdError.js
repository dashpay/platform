const ConsensusError = require('./ConsensusError');

class InvalidDocumentIdError extends ConsensusError {
  /**
   * @param {RawDocument} rawDocument
   */
  constructor(rawDocument) {
    super('Invalid Document ID');

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

module.exports = InvalidDocumentIdError;
