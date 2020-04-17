class MismatchContractIdsError extends Error {
  /**
   * @param {Document[]} documents
   */
  constructor(documents) {
    super();

    this.name = this.constructor.name;
    this.message = 'Documents have mixed contract ids';

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

module.exports = MismatchContractIdsError;
