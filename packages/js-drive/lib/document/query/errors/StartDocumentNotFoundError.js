const ValidationError = require('./ValidationError');

class StartDocumentNotFoundError extends ValidationError {
  /**
   * @param {string} field
   */
  constructor(field) {
    super(`Specified ${field} document not found`);

    this.field = field;
  }

  /**
   * Get field name
   *
   * @return {string}
   */
  getField() {
    return this.field;
  }

  /**
   * Get operators
   *
   * @return {Buffer}
   */
  getDocumentId() {
    return this.documentId;
  }
}

module.exports = StartDocumentNotFoundError;
