const ConsensusError = require('./ConsensusError');

class MismatchDocumentContractIdAndDataContractError extends ConsensusError {
  /**
   * @param {RawDocument} rawDocument
   * @param {DataContract} dataContract
   */
  constructor(rawDocument, dataContract) {
    super('Mismatch Document contract ID and Data Contract');

    this.dataContract = dataContract;
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

  /**
   * Get Data Contract
   *
   * @return {DataContract}
   */
  getDataContract() {
    return this.dataContract;
  }
}

module.exports = MismatchDocumentContractIdAndDataContractError;
