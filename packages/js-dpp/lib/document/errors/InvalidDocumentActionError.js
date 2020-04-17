class InvalidDocumentActionError extends Error {
  /**
   * @param {
   *   DocumentCreateTransition|DocumentReplaceTransition|DocumentDeleteTransition
   * } documentTransition
   */
  constructor(documentTransition) {
    super();

    this.name = this.constructor.name;
    this.message = `Invalid Document action ${documentTransition.getAction()}`;
    this.documentTransition = documentTransition;

    if (Error.captureStackTrace) {
      Error.captureStackTrace(this, this.constructor);
    }
  }

  /**
   * Get Document transition
   *
   * @return {
   *   DocumentCreateTransition|DocumentReplaceTransition|DocumentDeleteTransition
   * }
   */
  getDocumentTransition() {
    return this.documentTransition;
  }
}

module.exports = InvalidDocumentActionError;
