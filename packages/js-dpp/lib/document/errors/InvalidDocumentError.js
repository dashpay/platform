class InvalidDocumentError extends Error {
  /**
   * @param {ConsensusError[]} errors
   * @param {RawDocument} rawDocument
   */
  constructor(errors, rawDocument) {
    super();

    this.name = this.constructor.name;
    this.message = `Invalid Document: "${errors[0].message}"`;
    if (errors.length > 1) {
      this.message = `${this.message} and ${errors.length - 1} more`;
    }

    this.errors = errors;
    this.rawDocument = rawDocument;

    if (Error.captureStackTrace) {
      Error.captureStackTrace(this, this.constructor);
    }
  }

  /**
   * Get validation errors
   *
   * @return {ConsensusError[]}
   */
  getErrors() {
    return this.errors;
  }

  /**
   * Get raw Document
   *
   * @return {RawDocument}
   */
  getRawDocument() {
    return this.rawDocument;
  }
}

module.exports = InvalidDocumentError;
