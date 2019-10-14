const ConsensusError = require('./ConsensusError');

class InvalidDocumentEntropyError extends ConsensusError {
  /**
   * @param {RawDocument} rawDocument
   */
  constructor(rawDocument) {
    super('Invalid Document entropy');

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

module.exports = InvalidDocumentEntropyError;
