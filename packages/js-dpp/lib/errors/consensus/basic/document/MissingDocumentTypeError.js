const AbstractBasicError = require('../AbstractBasicError');

class MissingDocumentTypeError extends AbstractBasicError {
  /**
   * @param {RawDocument} rawDocument
   */
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
