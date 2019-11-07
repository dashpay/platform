const ValidationError = require('./ValidationError');

class InvalidDocumentTypeError extends ValidationError {
  /**
   *
   * @param {string} documentType
   */
  constructor(documentType) {
    super(`Invalid document type: ${documentType}`);

    this.documentType = documentType;
  }

  /**
   * Invalid document type
   *
   * @returns {string}
   */
  getDocumentType() {
    return this.documentType;
  }
}

module.exports = InvalidDocumentTypeError;
