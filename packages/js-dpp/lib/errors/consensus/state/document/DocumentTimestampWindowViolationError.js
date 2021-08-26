const AbstractStateError = require('../AbstractStateError');

class DocumentTimestampWindowViolationError extends AbstractStateError {
  /**
   * @param {string} timestampName
   * @param {DocumentCreateTransition
   *        |DocumentReplaceTransition} documentTransition
   * @param {Document} fetchedDocument
   */
  constructor(timestampName, documentTransition, fetchedDocument) {
    super(`Document ${timestampName} timestamp are out of block time window`);

    this.documentTransition = documentTransition;
    this.fetchedDocument = fetchedDocument;
    this.timestampName = timestampName;
  }

  /**
   * Get Document timestamp name
   *
   * @return {string}
   */
  getTimestampName() {
    return this.timestampName;
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

module.exports = DocumentTimestampWindowViolationError;
