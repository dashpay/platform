const ConsensusError = require('./ConsensusError');

class STDuplicateDocumentsError extends ConsensusError {
  /**
   * @param {RawDocument[]} duplicatedDocuments
   */
  constructor(duplicatedDocuments) {
    super('Duplicated Document in State Transition');

    this.duplicatedDocuments = duplicatedDocuments;
  }

  /**
   * Get Duplicated Documents
   *
   * @return {RawDocument[]}
   */
  getDuplicatedDocuments() {
    return this.duplicatedDocuments;
  }
}

module.exports = STDuplicateDocumentsError;
