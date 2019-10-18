const ConsensusError = require('./ConsensusError');

class STContainsDocumentsForDifferentDataContractsError extends ConsensusError {
  /**
   * @param {RawDocument[]} rawDocuments
   */
  constructor(rawDocuments) {
    super('State Transition contains documents for different Data Contracts');

    this.rawDocuments = rawDocuments;
  }

  /**
   * Get documents from different users
   *
   * @return {RawDocument[]}
   */
  getRawDocuments() {
    return this.rawDocuments;
  }
}

module.exports = STContainsDocumentsForDifferentDataContractsError;
