class MismatchOwnerIdsError extends Error {
  /**
   * @param {Document[]} documents
   */
  constructor(documents) {
    super();

    this.name = this.constructor.name;
    this.message = 'Documents have mixed owner ids';

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
