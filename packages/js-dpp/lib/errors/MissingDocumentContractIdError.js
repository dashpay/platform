const ConsensusError = require('./ConsensusError');

class MissingDocumentContractIdError extends ConsensusError {
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

module.exports = MissingDocumentContractIdError;
