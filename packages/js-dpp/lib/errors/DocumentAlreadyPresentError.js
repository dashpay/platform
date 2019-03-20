const ConsensusError = require('./ConsensusError');

class DocumentAlreadyPresentError extends ConsensusError {
  /**
   * @param {Document} document
   * @param {Document} fetchedDocument
   */
  constructor(document, fetchedDocument) {
    super('Document with the same ID is already present');

    this.document = document;
    this.fetchedDocument = fetchedDocument;
  }

  /**
   * Get Document
   *
   * @return {Document}
   */
  getDocument() {
    return this.document;
  }

  /**
   * Get fetched Document
   *
   * @return {Document}
   */
  getFetchedDocument() {
    return this.fetchedDocument;
  }
}

module.exports = DocumentAlreadyPresentError;
