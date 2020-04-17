const ConsensusError = require('./ConsensusError');

class DocumentOwnerIdMismatchError extends ConsensusError {
  /**
   * @param {DocumentReplaceTransition
   *        |DocumentDeleteTransition} documentTransition
   * @param {Document} fetchedDocument
   */
  constructor(documentTransition, fetchedDocument) {
    super('Document owner id mismatch with previous versions');

    this.documentTransition = documentTransition;
    this.fetchedDocument = fetchedDocument;
  }

  /**
   * Get Document Action Transition
   *
   * @return {DocumentReplaceTransition|DocumentDeleteTransition}
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

module.exports = DocumentOwnerIdMismatchError;
