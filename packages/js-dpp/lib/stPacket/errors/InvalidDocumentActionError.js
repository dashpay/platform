class InvalidDocumentActionError extends Error {
  /**
   * @param {Document} document
   */
  constructor(document) {
    super();

    this.name = this.constructor.name;
    this.message = 'Invalid Document action';
    this.document = document;

    if (Error.captureStackTrace) {
      Error.captureStackTrace(this, this.constructor);
    }
  }

  /**
   * Get Document
   *
   * @return {Document}
   */
  getDocument() {
    return this.document;
  }
}

module.exports = InvalidDocumentActionError;
