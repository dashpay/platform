const ValidationError = require('./ValidationError');

class NotIndexedFieldError extends ValidationError {
  /**
   *
   * @param {string} field
   */
  constructor(field) {
    super(`Query by not indexed field "${field}" is not allowed`);

    this.field = field;
  }

  /**
   * Get not indexed field
   *
   * @returns {string}
   */
  getNotIndexedField() {
    return this.field;
  }
}

module.exports = NotIndexedFieldError;
