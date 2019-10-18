const ConsensusError = require('./ConsensusError');

class STContainsDocumentsFromDifferentUsersError extends ConsensusError {
  /**
   * @param {RawDocument[]} rawDocuments
   */
  constructor(rawDocuments) {
    super('State Transition contains documents from different users');

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

module.exports = STContainsDocumentsFromDifferentUsersError;
