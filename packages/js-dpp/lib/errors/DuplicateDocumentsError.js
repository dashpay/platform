const ConsensusError = require('./ConsensusError');

class DuplicateDocumentsError extends ConsensusError {
  /**
   * @param {RawDocument[]} duplicatedDocuments
   */
  constructor(duplicatedDocuments) {
    super('Duplicated Document in ST Packet');

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

module.exports = DuplicateDocumentsError;
