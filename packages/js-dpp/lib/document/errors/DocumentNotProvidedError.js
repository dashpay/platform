class DocumentNotProvidedError extends Error {
  /**
   * @param {DocumentCreateTransition} documentTransition
   */
  constructor(documentTransition) {
    super();

    this.name = this.constructor.name;
    this.message = 'Document was not provided for apply of state transition';

    this.documentTransition = documentTransition;

    if (Error.captureStackTrace) {
      Error.captureStackTrace(this, this.constructor);
    }
  }

  /**
   * Get document transition
   *
   * @return {DocumentCreateTransition}
   */
  getDocumentTransition() {
    return this.documentTransition;
  }
}

module.exports = DocumentNotProvidedError;
