const ConsensusError = require('./ConsensusError');

class MissingDataContractIdError extends ConsensusError {
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
