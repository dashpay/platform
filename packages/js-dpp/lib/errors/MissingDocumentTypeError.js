const ConsensusError = require('./ConsensusError');

class MissingDocumentTypeError extends ConsensusError {
  constructor(rawDocument) {
    super('$type is not present');

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

module.exports = MissingDocumentTypeError;
