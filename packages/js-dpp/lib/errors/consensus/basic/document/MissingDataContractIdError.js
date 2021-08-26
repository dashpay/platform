const AbstractBasicError = require('../AbstractBasicError');

class MissingDataContractIdError extends AbstractBasicError {
  /**
   * @param {RawDocument} rawDocument
   */
  constructor(rawDocument) {
    super('$dataContractId is not present');

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

module.exports = MissingDataContractIdError;
