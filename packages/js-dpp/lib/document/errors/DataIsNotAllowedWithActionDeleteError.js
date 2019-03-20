class DataIsNotAllowedWithActionDeleteError extends Error {
  /**
   * @param {Document} document
   */
  constructor(document) {
    super();

    this.document = document;
    this.message = 'Data is not allowed for documents with $action DELETE';

    if (Error.captureStackTrace) {
      Error.captureStackTrace(this, this.constructor);
    }
  }

  /**
   * Get Document
   *
   * @returns {Document}
   */
  getDocument() {
    return this.document;
  }
}

module.exports = DataIsNotAllowedWithActionDeleteError;
