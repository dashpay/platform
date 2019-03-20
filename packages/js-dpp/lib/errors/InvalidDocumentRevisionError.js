const ConsensusError = require('./ConsensusError');

class InvalidDocumentRevisionError extends ConsensusError {
  /**
   * @param {Document} document
   * @param {Document} fetchedDocument
   */
  constructor(document, fetchedDocument) {
    super('Invalid Document revision');

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

module.exports = InvalidDocumentRevisionError;
