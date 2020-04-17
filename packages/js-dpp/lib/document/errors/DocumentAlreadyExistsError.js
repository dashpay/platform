class DocumentAlreadyExistsError extends Error {
  /**
   * @param {DocumentCreateTransition} documentTransition
   */
  constructor(documentTransition) {
    super();

    this.name = this.constructor.name;
    this.message = 'Document already exists';

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

module.exports = DocumentAlreadyExistsError;
