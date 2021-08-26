const DPPError = require('../../errors/DPPError');

class MismatchOwnerIdsError extends DPPError {
  /**
   * @param {Document[]} documents
   */
  constructor(documents) {
    super('Documents have mixed owner ids');

    this.documents = documents;
  }

  /**
   * Get documents
   *
   * @returns {Document[]}
   */
  getDocuments() {
    return this.documents;
  }
}

module.exports = MismatchOwnerIdsError;
