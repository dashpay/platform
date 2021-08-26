const DPPError = require('../../errors/DPPError');

class InvalidDocumentError extends DPPError {
  /**
   * @param {ConsensusError[]} errors
   * @param {RawDocument} rawDocument
   */
  constructor(errors, rawDocument) {
    let message = `Invalid Document: "${errors[0].message}"`;
    if (errors.length > 1) {
      message = `${message} and ${errors.length - 1} more`;
    }

    super(message);

    this.errors = errors;
    this.rawDocument = rawDocument;
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
