const ConsensusError = require('./ConsensusError');

class DocumentAlreadyPresentError extends ConsensusError {
  /**
   * @param {DocumentCreateTransition} documentTransition
   * @param {Document} fetchedDocument
   */
  constructor(documentTransition, fetchedDocument) {
    super('Document with the same ID is already present');

    this.documentTransition = documentTransition;
    this.fetchedDocument = fetchedDocument;
  }

  /**
   * Get document create transition
   *
   * @return {DocumentCreateTransition}
   */
  getDocumentTransition() {
    return this.documentTransition;
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
